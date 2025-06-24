use actix_web::{web, HttpResponse, Responder, HttpRequest};
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Postgres, Row};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use image::GenericImageView;

use crate::auth::{JwtManager, Claims};
use crate::storage::factory::create_storage_provider_from_project;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn test_is_image_file() {
        // Test valid image extensions
        assert!(is_image_file("test.jpg"));
        assert!(is_image_file("test.JPEG"));
        assert!(is_image_file("test.png"));
        assert!(is_image_file("test.PNG"));
        assert!(is_image_file("test.gif"));
        assert!(is_image_file("test.bmp"));
        assert!(is_image_file("test.webp"));
        assert!(is_image_file("test.svg"));
        assert!(is_image_file("test.ico"));
        assert!(is_image_file("test.tiff"));
        assert!(is_image_file("test.TIF"));
        
        // Test with paths
        assert!(is_image_file("path/to/image.jpg"));
        assert!(is_image_file("/absolute/path/to/image.PNG"));
        
        // Test non-image files
        assert!(!is_image_file("test.txt"));
        assert!(!is_image_file("test.pdf"));
        assert!(!is_image_file("test.doc"));
        assert!(!is_image_file("test"));
        assert!(!is_image_file(""));
        
        // Test edge cases
        assert!(!is_image_file(".jpg")); // Hidden file with jpg extension
        assert!(is_image_file("file.with.multiple.dots.jpg"));
    }

    #[test]
    fn test_extract_task_name_from_file() {
        assert_eq!(extract_task_name_from_file("image.jpg"), "image");
        assert_eq!(extract_task_name_from_file("path/to/image.png"), "image");
        assert_eq!(extract_task_name_from_file("/absolute/path/image.gif"), "image");
        assert_eq!(extract_task_name_from_file("file.with.dots.jpg"), "file.with.dots");
        assert_eq!(extract_task_name_from_file("no_extension"), "no_extension");
        assert_eq!(extract_task_name_from_file(""), "");
        assert_eq!(extract_task_name_from_file("/path/to/.hidden"), ".hidden");
    }

    #[tokio::test]
    async fn test_get_image_dimensions() {
        use crate::storage::{StorageProvider, StorageError, StorageMetadata};
        use async_trait::async_trait;
        use image::{ImageBuffer, RgbImage};

        // Mock storage provider for testing
        struct MockStorageProvider {
            should_fail: bool,
        }

        #[async_trait]
        impl StorageProvider for MockStorageProvider {
            async fn upload(&self, _key: &str, _data: &[u8], _content_type: Option<&str>) -> Result<String, StorageError> {
                unimplemented!()
            }

            async fn download(&self, _key: &str) -> Result<Vec<u8>, StorageError> {
                if self.should_fail {
                    Err(StorageError::NotFound)
                } else {
                    // Create a valid 5x5 RGB PNG image using the image crate
                    let img: RgbImage = ImageBuffer::new(5, 5);
                    let mut buffer = Vec::new();
                    img.write_to(&mut std::io::Cursor::new(&mut buffer), image::ImageFormat::Png)
                        .expect("Failed to write PNG");
                    Ok(buffer)
                }
            }

            async fn get_presigned_url(&self, _key: &str, _expires_in_secs: u64) -> Result<String, StorageError> {
                unimplemented!()
            }

            async fn delete(&self, _key: &str) -> Result<(), StorageError> {
                unimplemented!()
            }

            async fn exists(&self, _key: &str) -> Result<bool, StorageError> {
                unimplemented!()
            }

            async fn get_metadata(&self, _key: &str) -> Result<StorageMetadata, StorageError> {
                unimplemented!()
            }

            async fn list_objects(&self, _prefix: Option<&str>) -> Result<Vec<String>, StorageError> {
                unimplemented!()
            }
        }

        // Test successful dimension retrieval
        let provider = MockStorageProvider { should_fail: false };
        let result = get_image_dimensions(&provider, "test.png").await;
        if let Err(e) = &result {
            println!("Error getting dimensions: {}", e);
        }
        assert!(result.is_ok());
        let (width, height) = result.unwrap();
        assert_eq!(width, 5);
        assert_eq!(height, 5);

        // Test download failure
        let provider = MockStorageProvider { should_fail: true };
        let result = get_image_dimensions(&provider, "test.png").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Failed to download image"));
    }
}

#[derive(Debug, Deserialize)]
pub struct SyncRequest {
    pub prefix: Option<String>,
    pub file_extensions: Option<Vec<String>>,
    pub overwrite_existing: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct SyncResponse {
    pub sync_id: Uuid,
    pub total_files: usize,
    pub tasks_created: usize,
    pub tasks_skipped: usize,
    pub errors: Vec<String>,
    pub started_at: DateTime<Utc>,
    pub completed_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct SyncStatus {
    pub sync_id: Uuid,
    pub project_id: Uuid,
    pub status: String, // 'running', 'completed', 'failed'
    pub total_files: usize,
    pub processed_files: usize,
    pub tasks_created: usize,
    pub tasks_skipped: usize,
    pub errors: Vec<String>,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

pub async fn sync_storage_to_tasks(
    req: HttpRequest,
    path: web::Path<String>,
    payload: web::Json<SyncRequest>,
    pool: web::Data<Pool<Postgres>>,
    config: web::Data<crate::auth::OAuthConfig>,
) -> impl Responder {
    let claims = match extract_user_claims(&req, &config) {
        Ok(claims) => claims,
        Err(response) => return response,
    };

    let user_id = match Uuid::parse_str(&claims.sub) {
        Ok(id) => id,
        Err(_) => return HttpResponse::BadRequest().json("Invalid user ID"),
    };

    let project_id = match Uuid::parse_str(&path.into_inner()) {
        Ok(id) => id,
        Err(_) => return HttpResponse::BadRequest().json("Invalid project ID"),
    };

    if !user_has_project_access(&pool, project_id, user_id).await {
        return HttpResponse::NotFound().json("Project not found or access denied");
    }

    let project = match get_project_by_id(&pool, project_id).await {
        Ok(Some(project)) => project,
        Ok(None) => return HttpResponse::NotFound().json("Project not found"),
        Err(_) => return HttpResponse::InternalServerError().json("Failed to fetch project"),
    };

    if project.storage_config.is_none() {
        return HttpResponse::BadRequest().json("Project has no storage configuration");
    }

    let storage_provider = match create_storage_provider_from_project(&project).await {
        Ok(provider) => provider,
        Err(e) => return HttpResponse::InternalServerError().json(format!("Storage error: {}", e)),
    };

    let sync_id = Uuid::new_v4();
    let started_at = Utc::now();

    // Record sync start
    if let Err(e) = record_sync_start(&pool, sync_id, project_id, &started_at).await {
        return HttpResponse::InternalServerError().json(format!("Failed to record sync start: {}", e));
    }

    // Get files from storage
    let files = match storage_provider.list_objects(payload.prefix.as_deref()).await {
        Ok(files) => files,
        Err(e) => {
            let _ = record_sync_error(&pool, sync_id, &format!("Failed to list storage objects: {}", e)).await;
            return HttpResponse::InternalServerError().json(format!("Failed to list storage objects: {}", e));
        }
    };

    // Filter files by extension if specified
    let filtered_files = if let Some(extensions) = &payload.file_extensions {
        files.into_iter()
            .filter(|file| {
                if let Some(ext) = std::path::Path::new(file).extension() {
                    if let Some(ext_str) = ext.to_str() {
                        return extensions.iter().any(|allowed_ext| 
                            allowed_ext.trim_start_matches('.').eq_ignore_ascii_case(ext_str)
                        );
                    }
                }
                false
            })
            .collect::<Vec<_>>()
    } else {
        files
    };

    let total_files = filtered_files.len();
    let mut tasks_created = 0;
    let mut tasks_skipped = 0;
    let mut errors = Vec::new();

    // Update sync status with total files
    if let Err(e) = update_sync_progress(&pool, sync_id, total_files, 0, 0, 0).await {
        errors.push(format!("Failed to update sync progress: {}", e));
    }

    // Create tasks for each file
    for (index, file_key) in filtered_files.iter().enumerate() {
        let task_name = extract_task_name_from_file(file_key);
        let resource_url = format!("storage://{}", file_key);

        // Check if task already exists (unless overwrite is enabled)
        if !payload.overwrite_existing.unwrap_or(false) {
            if let Ok(true) = task_exists_for_resource(&pool, project_id, &resource_url).await {
                tasks_skipped += 1;
                continue;
            }
        }

        // Get image dimensions if it's an image file
        let dimensions = if is_image_file(file_key) {
            match get_image_dimensions(&*storage_provider, file_key).await {
                Ok(dims) => Some(dims),
                Err(e) => {
                    errors.push(format!("Failed to get dimensions for {}: {}", file_key, e));
                    None
                }
            }
        } else {
            None
        };

        match create_task_for_file(&pool, project_id, &task_name, &resource_url, dimensions).await {
            Ok(_) => tasks_created += 1,
            Err(e) => {
                errors.push(format!("Failed to create task for {}: {}", file_key, e));
                if errors.len() > 10 { // Limit error collection
                    errors.push("... and more errors".to_string());
                    break;
                }
            }
        }

        // Update progress periodically
        if index % 10 == 0 || index == filtered_files.len() - 1 {
            if let Err(e) = update_sync_progress(&pool, sync_id, total_files, index + 1, tasks_created, tasks_skipped).await {
                errors.push(format!("Failed to update sync progress: {}", e));
            }
        }
    }

    let completed_at = Utc::now();

    // Record sync completion
    if let Err(e) = record_sync_completion(&pool, sync_id, tasks_created, tasks_skipped, &errors, &completed_at).await {
        return HttpResponse::InternalServerError().json(format!("Failed to record sync completion: {}", e));
    }

    HttpResponse::Ok().json(SyncResponse {
        sync_id,
        total_files,
        tasks_created,
        tasks_skipped,
        errors,
        started_at,
        completed_at,
    })
}

pub async fn get_sync_status(
    req: HttpRequest,
    path: web::Path<(String, String)>,
    pool: web::Data<Pool<Postgres>>,
    config: web::Data<crate::auth::OAuthConfig>,
) -> impl Responder {
    let claims = match extract_user_claims(&req, &config) {
        Ok(claims) => claims,
        Err(response) => return response,
    };

    let user_id = match Uuid::parse_str(&claims.sub) {
        Ok(id) => id,
        Err(_) => return HttpResponse::BadRequest().json("Invalid user ID"),
    };

    let (project_id_str, sync_id_str) = path.into_inner();
    let project_id = match Uuid::parse_str(&project_id_str) {
        Ok(id) => id,
        Err(_) => return HttpResponse::BadRequest().json("Invalid project ID"),
    };

    let sync_id = match Uuid::parse_str(&sync_id_str) {
        Ok(id) => id,
        Err(_) => return HttpResponse::BadRequest().json("Invalid sync ID"),
    };

    if !user_has_project_access(&pool, project_id, user_id).await {
        return HttpResponse::NotFound().json("Project not found or access denied");
    }

    match get_sync_status_from_db(&pool, sync_id, project_id).await {
        Ok(Some(status)) => HttpResponse::Ok().json(status),
        Ok(None) => HttpResponse::NotFound().json("Sync not found"),
        Err(_) => HttpResponse::InternalServerError().json("Failed to fetch sync status"),
    }
}

fn is_image_file(file_key: &str) -> bool {
    if let Some(ext) = std::path::Path::new(file_key).extension() {
        if let Some(ext_str) = ext.to_str() {
            let image_extensions = ["jpg", "jpeg", "png", "gif", "bmp", "webp", "svg", "ico", "tiff", "tif"];
            return image_extensions.iter().any(|&img_ext| img_ext.eq_ignore_ascii_case(ext_str));
        }
    }
    false
}

async fn get_image_dimensions(
    storage_provider: &dyn crate::storage::StorageProvider,
    file_key: &str,
) -> Result<(u32, u32), String> {
    // Download the image
    let image_data = storage_provider.download(file_key)
        .await
        .map_err(|e| format!("Failed to download image: {}", e))?;
    
    // Load the image and get dimensions
    let img = image::load_from_memory(&image_data)
        .map_err(|e| format!("Failed to parse image: {}", e))?;
    
    let (width, height) = img.dimensions();
    Ok((width, height))
}

async fn record_sync_start(
    pool: &Pool<Postgres>,
    sync_id: Uuid,
    project_id: Uuid,
    started_at: &DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO project_syncs (id, project_id, status, total_files, processed_files, tasks_created, tasks_skipped, errors, started_at)
        VALUES ($1, $2, 'running', 0, 0, 0, 0, '[]'::jsonb, $3)
        "#
    )
    .bind(sync_id)
    .bind(project_id)
    .bind(started_at)
    .execute(pool)
    .await?;

    Ok(())
}

async fn update_sync_progress(
    pool: &Pool<Postgres>,
    sync_id: Uuid,
    total_files: usize,
    processed_files: usize,
    tasks_created: usize,
    tasks_skipped: usize,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        UPDATE project_syncs 
        SET total_files = $1, processed_files = $2, tasks_created = $3, tasks_skipped = $4
        WHERE id = $5
        "#
    )
    .bind(total_files as i32)
    .bind(processed_files as i32)
    .bind(tasks_created as i32)
    .bind(tasks_skipped as i32)
    .bind(sync_id)
    .execute(pool)
    .await?;

    Ok(())
}

async fn record_sync_completion(
    pool: &Pool<Postgres>,
    sync_id: Uuid,
    tasks_created: usize,
    tasks_skipped: usize,
    errors: &[String],
    completed_at: &DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    let status = if errors.is_empty() { "completed" } else { "completed_with_errors" };
    let errors_json = serde_json::to_value(errors).unwrap_or_default();

    sqlx::query(
        r#"
        UPDATE project_syncs 
        SET status = $1, tasks_created = $2, tasks_skipped = $3, errors = $4, completed_at = $5
        WHERE id = $6
        "#
    )
    .bind(status)
    .bind(tasks_created as i32)
    .bind(tasks_skipped as i32)
    .bind(errors_json)
    .bind(completed_at)
    .bind(sync_id)
    .execute(pool)
    .await?;

    Ok(())
}

async fn record_sync_error(
    pool: &Pool<Postgres>,
    sync_id: Uuid,
    error: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        UPDATE project_syncs 
        SET status = 'failed', errors = jsonb_build_array($1), completed_at = NOW()
        WHERE id = $2
        "#
    )
    .bind(error)
    .bind(sync_id)
    .execute(pool)
    .await?;

    Ok(())
}

async fn get_sync_status_from_db(
    pool: &Pool<Postgres>,
    sync_id: Uuid,
    project_id: Uuid,
) -> Result<Option<SyncStatus>, sqlx::Error> {
    let row = sqlx::query(
        r#"
        SELECT id, project_id, status, total_files, processed_files, tasks_created, tasks_skipped, errors, started_at, completed_at
        FROM project_syncs 
        WHERE id = $1 AND project_id = $2
        "#
    )
    .bind(sync_id)
    .bind(project_id)
    .fetch_optional(pool)
    .await?;

    if let Some(row) = row {
        let errors: Vec<String> = serde_json::from_value(row.get("errors")).unwrap_or_default();
        
        Ok(Some(SyncStatus {
            sync_id: row.get("id"),
            project_id: row.get("project_id"),
            status: row.get("status"),
            total_files: row.get::<i32, _>("total_files") as usize,
            processed_files: row.get::<i32, _>("processed_files") as usize,
            tasks_created: row.get::<i32, _>("tasks_created") as usize,
            tasks_skipped: row.get::<i32, _>("tasks_skipped") as usize,
            errors,
            started_at: row.get("started_at"),
            completed_at: row.get("completed_at"),
        }))
    } else {
        Ok(None)
    }
}

async fn task_exists_for_resource(
    pool: &Pool<Postgres>,
    project_id: Uuid,
    resource_url: &str,
) -> Result<bool, sqlx::Error> {
    let exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM tasks WHERE project_id = $1 AND resource_url = $2)"
    )
    .bind(project_id)
    .bind(resource_url)
    .fetch_one(pool)
    .await?;

    Ok(exists)
}

async fn create_task_for_file(
    pool: &Pool<Postgres>,
    project_id: Uuid,
    name: &str,
    resource_url: &str,
    dimensions: Option<(u32, u32)>,
) -> Result<(), sqlx::Error> {
    let task_id = Uuid::new_v4();
    let now = Utc::now();

    sqlx::query(
        r#"
        INSERT INTO tasks (id, project_id, name, resource_url, status, width, height, created_at, updated_at)
        VALUES ($1, $2, $3, $4, 'pending', $5, $6, $7, $8)
        "#
    )
    .bind(task_id)
    .bind(project_id)
    .bind(name)
    .bind(resource_url)
    .bind(dimensions.map(|(w, _)| w as i32))
    .bind(dimensions.map(|(_, h)| h as i32))
    .bind(now)
    .bind(now)
    .execute(pool)
    .await?;

    Ok(())
}

fn extract_task_name_from_file(file_key: &str) -> String {
    std::path::Path::new(file_key)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or(file_key)
        .to_string()
}

async fn user_has_project_access(pool: &Pool<Postgres>, project_id: Uuid, user_id: Uuid) -> bool {
    sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS(
            SELECT 1 FROM project_members pm
            WHERE pm.project_id = $1 AND pm.user_id = $2
        )
        "#
    )
    .bind(project_id)
    .bind(user_id)
    .fetch_one(pool)
    .await
    .unwrap_or(false)
}

async fn get_project_by_id(
    pool: &Pool<Postgres>,
    project_id: Uuid,
) -> Result<Option<crate::projects::Project>, sqlx::Error> {
    sqlx::query_as::<_, crate::projects::Project>(
        "SELECT id, name, description, storage_config, owner_id, created_at, updated_at FROM projects WHERE id = $1"
    )
    .bind(project_id)
    .fetch_optional(pool)
    .await
}

fn extract_user_claims(
    req: &HttpRequest,
    config: &crate::auth::OAuthConfig,
) -> Result<Claims, HttpResponse> {
    let auth_header = match req.headers().get("Authorization") {
        Some(header) => header,
        None => return Err(HttpResponse::Unauthorized().json("Authorization header missing")),
    };

    let auth_str = match auth_header.to_str() {
        Ok(str) => str,
        Err(_) => return Err(HttpResponse::Unauthorized().json("Invalid authorization header")),
    };

    let token = match auth_str.strip_prefix("Bearer ") {
        Some(token) => token,
        None => return Err(HttpResponse::Unauthorized().json("Invalid authorization format")),
    };

    let jwt_manager = JwtManager::new(&config.jwt_secret);
    match jwt_manager.verify_token(token) {
        Ok(claims) => Ok(claims),
        Err(_) => Err(HttpResponse::Unauthorized().json("Invalid or expired token")),
    }
}