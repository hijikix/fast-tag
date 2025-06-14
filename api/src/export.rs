use actix_web::{web, HttpResponse, Responder, HttpRequest};
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Postgres};
use uuid::Uuid;
use chrono::{Datelike, Utc};
use actix_multipart::Multipart;
use futures_util::TryStreamExt;

use crate::auth::{JwtManager, Claims};

// COCO format data structures
#[derive(Debug, Serialize, Deserialize)]
pub struct CocoExport {
    pub info: CocoInfo,
    pub images: Vec<CocoImage>,
    pub annotations: Vec<CocoAnnotation>,
    pub categories: Vec<CocoCategory>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CocoInfo {
    pub year: i32,
    pub version: String,
    pub description: String,
    pub contributor: String,
    pub url: String,
    pub date_created: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CocoImage {
    pub id: i64,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub file_name: String,
    pub license: Option<i32>,
    pub flickr_url: Option<String>,
    pub coco_url: Option<String>,
    pub date_captured: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CocoAnnotation {
    pub id: i64,
    pub image_id: i64,
    pub category_id: i32,
    pub segmentation: Vec<Vec<f64>>,
    pub area: f64,
    pub bbox: Vec<f64>,
    pub iscrowd: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CocoCategory {
    pub id: i32,
    pub name: String,
    pub supercategory: String,
}

// Import specific structures
#[derive(Debug, Deserialize)]
pub struct CocoImport {
    #[allow(dead_code)]
    pub info: Option<CocoInfo>,
    pub images: Vec<CocoImage>,
    pub annotations: Vec<CocoAnnotation>,
    pub categories: Vec<CocoCategory>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ImportResult {
    pub success: bool,
    pub message: String,
    pub stats: ImportStats,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ImportStats {
    pub categories_created: usize,
    pub categories_updated: usize,
    pub tasks_created: usize,
    pub annotations_created: usize,
    pub errors: Vec<String>,
}

pub async fn import_project_coco(
    req: HttpRequest,
    path: web::Path<String>,
    mut payload: Multipart,
    pool: web::Data<Pool<Postgres>>,
    config: web::Data<crate::auth::OAuthConfig>,
) -> impl Responder {
    // Extract and verify JWT token
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

    // Check if user has access to this project
    if !user_has_project_access(&pool, project_id, user_id).await {
        return HttpResponse::NotFound().json("Project not found or access denied");
    }

    // Extract JSON data from multipart upload
    let json_data = match extract_json_from_multipart(&mut payload).await {
        Ok(data) => data,
        Err(err) => return HttpResponse::BadRequest().json(format!("Failed to read file: {}", err)),
    };

    // Parse COCO JSON
    let coco_data: CocoImport = match serde_json::from_str(&json_data) {
        Ok(data) => data,
        Err(err) => return HttpResponse::BadRequest().json(format!("Invalid COCO JSON: {}", err)),
    };

    // Validate COCO data
    if let Err(validation_error) = validate_coco_data(&coco_data) {
        return HttpResponse::BadRequest().json(format!("Invalid COCO data: {}", validation_error));
    }

    // Import the data
    match import_coco_data(&pool, project_id, user_id, coco_data).await {
        Ok(result) => HttpResponse::Ok().json(result),
        Err(err) => {
            eprintln!("Import error: {:?}", err);
            HttpResponse::InternalServerError().json("Failed to import COCO data")
        }
    }
}

async fn extract_json_from_multipart(payload: &mut Multipart) -> Result<String, Box<dyn std::error::Error>> {
    while let Some(mut field) = payload.try_next().await? {
        let field_name = field.name();
        
        if field_name == Some("file") {
            let mut data = bytes::BytesMut::new();
            while let Some(chunk) = field.try_next().await? {
                data.extend_from_slice(&chunk);
            }
            
            return Ok(String::from_utf8(data.to_vec())?);
        }
    }
    
    Err("No file field found in multipart data".into())
}

fn validate_coco_data(coco_data: &CocoImport) -> Result<(), String> {
    // Check for required fields
    if coco_data.categories.is_empty() {
        return Err("No categories found in COCO data".to_string());
    }

    if coco_data.images.is_empty() {
        return Err("No images found in COCO data".to_string());
    }

    // Validate category IDs are unique
    let mut category_ids = std::collections::HashSet::new();
    for category in &coco_data.categories {
        if !category_ids.insert(category.id) {
            return Err(format!("Duplicate category ID: {}", category.id));
        }
    }

    // Validate image IDs are unique
    let mut image_ids = std::collections::HashSet::new();
    for image in &coco_data.images {
        if !image_ids.insert(image.id) {
            return Err(format!("Duplicate image ID: {}", image.id));
        }
    }

    // Validate annotations reference valid categories and images
    let category_id_set: std::collections::HashSet<_> = coco_data.categories.iter().map(|c| c.id).collect();
    let image_id_set: std::collections::HashSet<_> = coco_data.images.iter().map(|i| i.id).collect();

    for annotation in &coco_data.annotations {
        if !category_id_set.contains(&annotation.category_id) {
            return Err(format!("Annotation {} references invalid category ID: {}", annotation.id, annotation.category_id));
        }
        if !image_id_set.contains(&annotation.image_id) {
            return Err(format!("Annotation {} references invalid image ID: {}", annotation.id, annotation.image_id));
        }
        if annotation.bbox.len() != 4 {
            return Err(format!("Annotation {} has invalid bbox format", annotation.id));
        }
    }

    Ok(())
}

async fn import_coco_data(
    pool: &Pool<Postgres>,
    project_id: Uuid,
    user_id: Uuid,
    coco_data: CocoImport,
) -> Result<ImportResult, sqlx::Error> {
    let mut stats = ImportStats {
        categories_created: 0,
        categories_updated: 0,
        tasks_created: 0,
        annotations_created: 0,
        errors: Vec::new(),
    };

    // Start transaction
    let mut tx = pool.begin().await?;

    // Import categories
    let mut category_mapping = std::collections::HashMap::new();
    for coco_category in &coco_data.categories {
        match import_category(&mut tx, project_id, coco_category).await {
            Ok((category_id, was_created)) => {
                category_mapping.insert(coco_category.id, category_id);
                if was_created {
                    stats.categories_created += 1;
                } else {
                    stats.categories_updated += 1;
                }
            }
            Err(err) => {
                stats.errors.push(format!("Failed to import category '{}': {}", coco_category.name, err));
            }
        }
    }

    // Import images as tasks
    let mut image_mapping = std::collections::HashMap::new();
    for coco_image in &coco_data.images {
        match import_image_as_task(&mut tx, project_id, coco_image).await {
            Ok(task_id) => {
                image_mapping.insert(coco_image.id, task_id);
                stats.tasks_created += 1;
            }
            Err(err) => {
                stats.errors.push(format!("Failed to import image '{}': {}", coco_image.file_name, err));
            }
        }
    }

    // Import annotations
    for coco_annotation in &coco_data.annotations {
        if let (Some(&category_id), Some(&task_id)) = (
            category_mapping.get(&coco_annotation.category_id),
            image_mapping.get(&coco_annotation.image_id),
        ) {
            match import_annotation(&mut tx, task_id, category_id, coco_annotation, user_id).await {
                Ok(_) => {
                    stats.annotations_created += 1;
                }
                Err(err) => {
                    stats.errors.push(format!("Failed to import annotation {}: {}", coco_annotation.id, err));
                }
            }
        } else {
            stats.errors.push(format!("Annotation {} references invalid category or image", coco_annotation.id));
        }
    }

    // Commit transaction
    tx.commit().await?;

    Ok(ImportResult {
        success: stats.errors.is_empty(),
        message: if stats.errors.is_empty() {
            "Import completed successfully".to_string()
        } else {
            format!("Import completed with {} errors", stats.errors.len())
        },
        stats,
    })
}

async fn import_category(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    project_id: Uuid,
    coco_category: &CocoCategory,
) -> Result<(Uuid, bool), sqlx::Error> {
    // Check if category already exists by name
    let existing_category = sqlx::query!(
        "SELECT id FROM image_annotation_categories WHERE project_id = $1 AND name = $2",
        project_id,
        coco_category.name
    )
    .fetch_optional(&mut **tx)
    .await?;

    match existing_category {
        Some(existing) => {
            // Update existing category
            sqlx::query!(
                r#"
                UPDATE image_annotation_categories 
                SET supercategory = $1, coco_id = $2, updated_at = NOW()
                WHERE id = $3
                "#,
                Some(coco_category.supercategory.clone()),
                Some(coco_category.id),
                existing.id
            )
            .execute(&mut **tx)
            .await?;
            
            Ok((existing.id, false))
        }
        None => {
            // Create new category
            let new_id = Uuid::new_v4();
            sqlx::query!(
                r#"
                INSERT INTO image_annotation_categories 
                (id, project_id, name, supercategory, coco_id, created_at, updated_at)
                VALUES ($1, $2, $3, $4, $5, NOW(), NOW())
                "#,
                new_id,
                project_id,
                coco_category.name,
                Some(coco_category.supercategory.clone()),
                Some(coco_category.id)
            )
            .execute(&mut **tx)
            .await?;
            
            Ok((new_id, true))
        }
    }
}

async fn import_image_as_task(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    project_id: Uuid,
    coco_image: &CocoImage,
) -> Result<Uuid, sqlx::Error> {
    // Check if task already exists by file name
    let existing_task = sqlx::query!(
        "SELECT id FROM tasks WHERE project_id = $1 AND name = $2",
        project_id,
        coco_image.file_name
    )
    .fetch_optional(&mut **tx)
    .await?;

    match existing_task {
        Some(existing) => {
            // Update existing task with resource URL if available
            if let Some(ref url) = coco_image.coco_url {
                sqlx::query!(
                    "UPDATE tasks SET resource_url = $1, updated_at = NOW() WHERE id = $2",
                    url,
                    existing.id
                )
                .execute(&mut **tx)
                .await?;
            }
            Ok(existing.id)
        }
        None => {
            // Create new task
            let new_id = Uuid::new_v4();
            sqlx::query!(
                r#"
                INSERT INTO tasks (id, project_id, name, resource_url, created_at, updated_at)
                VALUES ($1, $2, $3, $4, NOW(), NOW())
                "#,
                new_id,
                project_id,
                coco_image.file_name,
                coco_image.coco_url
            )
            .execute(&mut **tx)
            .await?;
            
            Ok(new_id)
        }
    }
}

async fn import_annotation(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    task_id: Uuid,
    category_id: Uuid,
    coco_annotation: &CocoAnnotation,
    user_id: Uuid,
) -> Result<Uuid, sqlx::Error> {
    let annotation_id = Uuid::new_v4();
    let image_annotation_id = Uuid::new_v4();
    let now = chrono::Utc::now();

    // Create annotation
    sqlx::query!(
        r#"
        INSERT INTO annotations (id, task_id, metadata, annotated_by, annotated_at, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        "#,
        annotation_id,
        task_id,
        serde_json::json!({"imported_from_coco": true, "original_coco_id": coco_annotation.id}),
        user_id,
        now,
        now,
        now
    )
    .execute(&mut **tx)
    .await?;

    // Create image annotation
    sqlx::query!(
        r#"
        INSERT INTO image_annotations (id, annotation_id, category_id, bbox, area, iscrowd, image_metadata, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        "#,
        image_annotation_id,
        annotation_id,
        Some(category_id),
        &coco_annotation.bbox,
        Some(coco_annotation.area),
        coco_annotation.iscrowd == 1,
        serde_json::json!({"imported_from_coco": true}),
        now,
        now
    )
    .execute(&mut **tx)
    .await?;

    Ok(annotation_id)
}

pub async fn export_project_coco(
    req: HttpRequest,
    path: web::Path<String>,
    pool: web::Data<Pool<Postgres>>,
    config: web::Data<crate::auth::OAuthConfig>,
) -> impl Responder {
    // Extract and verify JWT token
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

    // Check if user has access to this project
    if !user_has_project_access(&pool, project_id, user_id).await {
        return HttpResponse::NotFound().json("Project not found or access denied");
    }

    // Get project info
    let project = match get_project_info(&pool, project_id).await {
        Ok(Some(project)) => project,
        Ok(None) => return HttpResponse::NotFound().json("Project not found"),
        Err(_) => return HttpResponse::InternalServerError().json("Failed to fetch project"),
    };

    // Get categories
    let categories = match get_project_categories_for_export(&pool, project_id).await {
        Ok(cats) => cats,
        Err(_) => return HttpResponse::InternalServerError().json("Failed to fetch categories"),
    };

    // Get tasks with annotations
    let (images, annotations) = match get_project_annotations_for_export(&pool, project_id).await {
        Ok(data) => data,
        Err(_) => return HttpResponse::InternalServerError().json("Failed to fetch annotations"),
    };

    // Build COCO format export
    let coco_export = CocoExport {
        info: CocoInfo {
            year: Utc::now().year(),
            version: "1.0".to_string(),
            description: project.description.unwrap_or_else(|| project.name.clone()),
            contributor: claims.email,
            url: "https://fast-tag.com".to_string(),
            date_created: Utc::now().to_rfc3339(),
        },
        images,
        annotations,
        categories,
    };

    // Generate filename
    let filename = format!("{}_coco_export_{}.json", 
        project.name.replace(" ", "_").to_lowercase(), 
        Utc::now().format("%Y%m%d_%H%M%S")
    );

    // Return JSON file as download with 2-space indentation
    let pretty_json = match serde_json::to_string_pretty(&coco_export) {
        Ok(json) => json.replace("    ", "  "), // Convert 4-space indent to 2-space indent
        Err(_) => return HttpResponse::InternalServerError().json("Failed to serialize JSON"),
    };
    
    HttpResponse::Ok()
        .content_type("application/json")
        .insert_header(("Content-Disposition", format!("attachment; filename=\"{}\"", filename)))
        .body(pretty_json)
}

// Helper structures
#[derive(Debug)]
struct ProjectInfo {
    name: String,
    description: Option<String>,
}

async fn get_project_info(
    pool: &Pool<Postgres>,
    project_id: Uuid,
) -> Result<Option<ProjectInfo>, sqlx::Error> {
    let row = sqlx::query!(
        "SELECT name, description FROM projects WHERE id = $1",
        project_id
    )
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|r| ProjectInfo {
        name: r.name,
        description: r.description,
    }))
}

async fn get_project_categories_for_export(
    pool: &Pool<Postgres>,
    project_id: Uuid,
) -> Result<Vec<CocoCategory>, sqlx::Error> {
    let rows = sqlx::query!(
        r#"
        SELECT id, name, COALESCE(supercategory, 'object') as supercategory, coco_id
        FROM image_annotation_categories
        WHERE project_id = $1
        ORDER BY coco_id, name
        "#,
        project_id
    )
    .fetch_all(pool)
    .await?;

    let mut categories = Vec::new();
    let mut coco_id_counter = 1;

    for row in rows {
        let coco_id = row.coco_id.unwrap_or_else(|| {
            let id = coco_id_counter;
            coco_id_counter += 1;
            id
        });

        categories.push(CocoCategory {
            id: coco_id,
            name: row.name,
            supercategory: row.supercategory.unwrap_or_else(|| "object".to_string()),
        });
    }

    Ok(categories)
}

async fn get_project_annotations_for_export(
    pool: &Pool<Postgres>,
    project_id: Uuid,
) -> Result<(Vec<CocoImage>, Vec<CocoAnnotation>), sqlx::Error> {
    // First, get all tasks for the project
    let tasks = sqlx::query!(
        r#"
        SELECT id, name, resource_url, created_at
        FROM tasks
        WHERE project_id = $1
        ORDER BY created_at
        "#,
        project_id
    )
    .fetch_all(pool)
    .await?;

    // Then get all annotations with categories
    let annotation_rows = sqlx::query!(
        r#"
        SELECT 
            a.id as annotation_id,
            a.task_id,
            ia.bbox,
            ia.area,
            ia.iscrowd,
            iac.coco_id as category_coco_id,
            iac.id as category_id
        FROM annotations a
        JOIN image_annotations ia ON a.id = ia.annotation_id
        JOIN image_annotation_categories iac ON ia.category_id = iac.id
        JOIN tasks t ON a.task_id = t.id
        WHERE t.project_id = $1
        ORDER BY a.created_at
        "#,
        project_id
    )
    .fetch_all(pool)
    .await?;

    let mut images = Vec::new();
    let mut annotations = Vec::new();
    let mut task_id_to_image_id = std::collections::HashMap::new();
    let mut image_id_counter = 1i64;
    let mut annotation_id_counter = 1i64;
    let mut category_coco_ids = std::collections::HashMap::new();
    let mut category_coco_id_counter = 1i32;

    // Process tasks as images
    for task in tasks {
        let image_id = image_id_counter;
        image_id_counter += 1;
        task_id_to_image_id.insert(task.id, image_id);

        // Extract filename from resource_url or use task name
        let file_name = task.resource_url
            .as_ref()
            .and_then(|url| url.split('/').last())
            .unwrap_or(&task.name)
            .to_string();

        images.push(CocoImage {
            id: image_id,
            width: None, // Can be added later if we store image dimensions
            height: None,
            file_name,
            license: None,
            flickr_url: None,
            coco_url: task.resource_url,
            date_captured: task.created_at.map(|dt| dt.to_rfc3339()).unwrap_or_default(),
        });
    }

    // Process annotations
    for row in annotation_rows {
        if let Some(&image_id) = task_id_to_image_id.get(&row.task_id) {
            // Get or assign COCO category ID
            let category_coco_id = if let Some(coco_id) = row.category_coco_id {
                coco_id
            } else {
                *category_coco_ids.entry(row.category_id).or_insert_with(|| {
                    let id = category_coco_id_counter;
                    category_coco_id_counter += 1;
                    id
                })
            };

            // Convert bbox to Vec<f64>
            let bbox_vec: Vec<f64> = row.bbox.into_iter().map(|v| v as f64).collect();
            
            // Calculate area if not provided
            let area = row.area.unwrap_or_else(|| {
                if bbox_vec.len() >= 4 {
                    bbox_vec[2] * bbox_vec[3] // width * height
                } else {
                    0.0
                }
            }) as f64;

            annotations.push(CocoAnnotation {
                id: annotation_id_counter,
                image_id,
                category_id: category_coco_id,
                segmentation: vec![], // Bounding box format doesn't use segmentation
                area,
                bbox: bbox_vec,
                iscrowd: if row.iscrowd.unwrap_or(false) { 1 } else { 0 },
            });

            annotation_id_counter += 1;
        }
    }

    Ok((images, annotations))
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::{User, OAuthConfig, AuthStorage, JwtManager};
    use crate::test_utils;
    use actix_web::{test, App, web};
    use serial_test::serial;

    fn create_test_oauth_config() -> OAuthConfig {
        OAuthConfig {
            google_client_id: "test_google_id".to_string(),
            google_client_secret: "test_google_secret".to_string(),
            google_redirect_url: "http://localhost/callback".to_string(),
            github_client_id: "test_github_id".to_string(),
            github_client_secret: "test_github_secret".to_string(),
            github_redirect_url: "http://localhost/callback".to_string(),
            jwt_secret: "test_jwt_secret_key_that_is_long_enough".to_string(),
        }
    }

    fn create_auth_token(oauth_config: &OAuthConfig, user: &User) -> String {
        let jwt_manager = JwtManager::new(&oauth_config.jwt_secret);
        jwt_manager.generate_token(
            &user.id.to_string(),
            &user.email,
            &user.name
        ).expect("Failed to generate token")
    }

    #[actix_web::test]
    #[serial]
    async fn test_export_project_coco_empty() {
        let pool = test_utils::setup_test_db().await;
        let user = test_utils::create_test_user_with_details(&pool).await;
        let oauth_config = create_test_oauth_config();
        let token = create_auth_token(&oauth_config, &user);
        let auth_storage = AuthStorage::new(pool.clone());

        // Create a test project
        let project = crate::projects::create_project_in_db(&pool, "Test Project", Some("Description"), None, user.id).await.unwrap();

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(pool.clone()))
                .app_data(web::Data::new(oauth_config))
                .app_data(web::Data::new(auth_storage))
                .route("/projects/{project_id}/export/coco", web::get().to(export_project_coco))
        ).await;

        let req = test::TestRequest::get()
            .uri(&format!("/projects/{}/export/coco", project.id))
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);

        let headers = resp.headers();
        assert!(headers.get("Content-Disposition").is_some());
        assert!(headers.get("Content-Type").unwrap().to_str().unwrap().contains("application/json"));

        let body: CocoExport = test::read_body_json(resp).await;
        assert_eq!(body.images.len(), 0);
        assert_eq!(body.annotations.len(), 0);
        assert_eq!(body.categories.len(), 0);
    }

    #[actix_web::test]
    #[serial]
    async fn test_export_project_coco_with_data() {
        let pool = test_utils::setup_test_db().await;
        let user = test_utils::create_test_user_with_details(&pool).await;
        let oauth_config = create_test_oauth_config();
        let token = create_auth_token(&oauth_config, &user);
        let auth_storage = AuthStorage::new(pool.clone());

        // Create test data
        let project = crate::projects::create_project_in_db(&pool, "Test Project", Some("Description"), None, user.id).await.unwrap();
        let category = crate::image_annotation_categories::create_image_annotation_category_in_db(&pool, project.id, "person", None, Some("human"), Some("#FF0000"), Some(1)).await.unwrap();
        let task = crate::tasks::create_task_in_db(&pool, project.id, "image1.jpg", Some("https://example.com/image1.jpg")).await.unwrap();
        
        // Create annotation
        crate::annotations::create_annotation_in_db(&pool, task.id, category.id, &[100.0, 50.0, 200.0, 150.0], Some(30000.0), false, &serde_json::json!({}), user.id).await.unwrap();

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(pool.clone()))
                .app_data(web::Data::new(oauth_config))
                .app_data(web::Data::new(auth_storage))
                .route("/projects/{project_id}/export/coco", web::get().to(export_project_coco))
        ).await;

        let req = test::TestRequest::get()
            .uri(&format!("/projects/{}/export/coco", project.id))
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);

        let body: CocoExport = test::read_body_json(resp).await;
        assert_eq!(body.images.len(), 1);
        assert_eq!(body.annotations.len(), 1);
        assert_eq!(body.categories.len(), 1);

        // Check category
        assert_eq!(body.categories[0].id, 1);
        assert_eq!(body.categories[0].name, "person");
        assert_eq!(body.categories[0].supercategory, "human");

        // Check image
        assert_eq!(body.images[0].file_name, "image1.jpg");
        assert_eq!(body.images[0].coco_url, Some("https://example.com/image1.jpg".to_string()));

        // Check annotation
        assert_eq!(body.annotations[0].category_id, 1);
        assert_eq!(body.annotations[0].bbox, vec![100.0, 50.0, 200.0, 150.0]);
        assert_eq!(body.annotations[0].area, 30000.0);
        assert_eq!(body.annotations[0].iscrowd, 0);
    }

    #[actix_web::test]
    #[serial]
    async fn test_export_project_coco_unauthorized() {
        let pool = test_utils::setup_test_db().await;
        let user = test_utils::create_test_user_with_details(&pool).await;
        let oauth_config = create_test_oauth_config();
        let auth_storage = AuthStorage::new(pool.clone());

        let project = crate::projects::create_project_in_db(&pool, "Test Project", Some("Description"), None, user.id).await.unwrap();

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(pool.clone()))
                .app_data(web::Data::new(oauth_config))
                .app_data(web::Data::new(auth_storage))
                .route("/projects/{project_id}/export/coco", web::get().to(export_project_coco))
        ).await;

        let req = test::TestRequest::get()
            .uri(&format!("/projects/{}/export/coco", project.id))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 401);
    }

    #[actix_web::test]
    #[serial]
    async fn test_import_project_coco_success() {
        let pool = test_utils::setup_test_db().await;
        let user = test_utils::create_test_user_with_details(&pool).await;
        let oauth_config = create_test_oauth_config();
        let token = create_auth_token(&oauth_config, &user);
        let auth_storage = AuthStorage::new(pool.clone());

        // Create a test project
        let project = crate::projects::create_project_in_db(&pool, "Test Project", Some("Description"), None, user.id).await.unwrap();

        // Create test COCO data
        let coco_data = serde_json::json!({
            "info": {
                "year": 2024,
                "version": "1.0",
                "description": "Test dataset",
                "contributor": "test@example.com",
                "url": "https://example.com",
                "date_created": "2024-01-01T00:00:00Z"
            },
            "images": [
                {
                    "id": 1,
                    "width": 640,
                    "height": 480,
                    "file_name": "test_image.jpg",
                    "license": null,
                    "flickr_url": null,
                    "coco_url": "https://example.com/test_image.jpg",
                    "date_captured": "2024-01-01T00:00:00Z"
                }
            ],
            "annotations": [
                {
                    "id": 1,
                    "image_id": 1,
                    "category_id": 1,
                    "segmentation": [],
                    "area": 30000.0,
                    "bbox": [100.0, 50.0, 200.0, 150.0],
                    "iscrowd": 0
                }
            ],
            "categories": [
                {
                    "id": 1,
                    "name": "person",
                    "supercategory": "human"
                }
            ]
        });

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(pool.clone()))
                .app_data(web::Data::new(oauth_config))
                .app_data(web::Data::new(auth_storage))
                .route("/projects/{project_id}/import/coco", web::post().to(import_project_coco))
        ).await;

        // Create multipart form data
        let json_str = serde_json::to_string(&coco_data).unwrap();
        let boundary = "----formdata-test-boundary";
        let body = format!(
            "--{}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"test.json\"\r\nContent-Type: application/json\r\n\r\n{}\r\n--{}--\r\n",
            boundary, json_str, boundary
        );

        let req = test::TestRequest::post()
            .uri(&format!("/projects/{}/import/coco", project.id))
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .insert_header(("content-type", format!("multipart/form-data; boundary={}", boundary)))
            .set_payload(body)
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);

        let body: ImportResult = test::read_body_json(resp).await;
        assert!(body.success);
        assert_eq!(body.stats.categories_created, 1);
        assert_eq!(body.stats.tasks_created, 1);
        assert_eq!(body.stats.annotations_created, 1);
    }

    #[actix_web::test]
    #[serial]
    async fn test_import_project_coco_invalid_json() {
        let pool = test_utils::setup_test_db().await;
        let user = test_utils::create_test_user_with_details(&pool).await;
        let oauth_config = create_test_oauth_config();
        let token = create_auth_token(&oauth_config, &user);
        let auth_storage = AuthStorage::new(pool.clone());

        let project = crate::projects::create_project_in_db(&pool, "Test Project", Some("Description"), None, user.id).await.unwrap();

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(pool.clone()))
                .app_data(web::Data::new(oauth_config))
                .app_data(web::Data::new(auth_storage))
                .route("/projects/{project_id}/import/coco", web::post().to(import_project_coco))
        ).await;

        let boundary = "----formdata-test-boundary";
        let body = format!(
            "--{}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"test.json\"\r\nContent-Type: application/json\r\n\r\n{{\r\n--{}--\r\n",
            boundary, boundary
        );

        let req = test::TestRequest::post()
            .uri(&format!("/projects/{}/import/coco", project.id))
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .insert_header(("content-type", format!("multipart/form-data; boundary={}", boundary)))
            .set_payload(body)
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 400);
    }

    #[actix_web::test]
    #[serial]
    async fn test_import_project_coco_unauthorized() {
        let pool = test_utils::setup_test_db().await;
        let user = test_utils::create_test_user_with_details(&pool).await;
        let oauth_config = create_test_oauth_config();
        let auth_storage = AuthStorage::new(pool.clone());

        let project = crate::projects::create_project_in_db(&pool, "Test Project", Some("Description"), None, user.id).await.unwrap();

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(pool.clone()))
                .app_data(web::Data::new(oauth_config))
                .app_data(web::Data::new(auth_storage))
                .route("/projects/{project_id}/import/coco", web::post().to(import_project_coco))
        ).await;

        let coco_data = serde_json::json!({
            "images": [],
            "annotations": [],
            "categories": []
        });

        let json_str = serde_json::to_string(&coco_data).unwrap();
        let boundary = "----formdata-test-boundary";
        let body = format!(
            "--{}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"test.json\"\r\nContent-Type: application/json\r\n\r\n{}\r\n--{}--\r\n",
            boundary, json_str, boundary
        );

        let req = test::TestRequest::post()
            .uri(&format!("/projects/{}/import/coco", project.id))
            .insert_header(("content-type", format!("multipart/form-data; boundary={}", boundary)))
            .set_payload(body)
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 401);
    }
}