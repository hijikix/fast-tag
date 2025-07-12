use actix_web::{web, HttpResponse, Responder, HttpRequest};
use actix_multipart::Multipart;
use futures_util::TryStreamExt;
use sqlx::{Pool, Postgres};
use uuid::Uuid;

use super::types::{CocoImport, CocoCategory, CocoImage, CocoAnnotation, ImportResult, ImportStats};
use super::export::{user_has_project_access, extract_user_claims};

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

    // Group annotations by task to ensure only one annotation per task
    let mut task_annotations: std::collections::HashMap<Uuid, Vec<(&CocoAnnotation, Uuid)>> = std::collections::HashMap::new();
    
    for coco_annotation in &coco_data.annotations {
        if let (Some(&category_id), Some(&task_id)) = (
            category_mapping.get(&coco_annotation.category_id),
            image_mapping.get(&coco_annotation.image_id),
        ) {
            task_annotations.entry(task_id).or_default().push((coco_annotation, category_id));
        } else {
            stats.errors.push(format!("Annotation {} references invalid category or image", coco_annotation.id));
        }
    }

    // Import annotations - one annotation per task with multiple image_annotations
    for (task_id, annotations) in task_annotations {
        println!("Processing task_id: {}, with {} COCO annotations", task_id, annotations.len());
        match import_task_annotations(&mut tx, task_id, &annotations, user_id).await {
            Ok(_annotation_count) => {
                println!("Successfully created 1 annotation for task_id: {}", task_id);
                stats.annotations_created += 1; // One annotation per task
            }
            Err(err) => {
                println!("Failed to import annotations for task {}: {}", task_id, err);
                stats.errors.push(format!("Failed to import annotations for task {}: {}", task_id, err));
            }
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

async fn import_task_annotations(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    task_id: Uuid,
    annotations: &[(&CocoAnnotation, Uuid)],
    user_id: Uuid,
) -> Result<usize, sqlx::Error> {
    let annotation_id = Uuid::new_v4();
    let now = chrono::Utc::now();

    // Collect all original COCO IDs for metadata
    let coco_ids: Vec<i64> = annotations.iter().map(|(ann, _)| ann.id).collect();
    
    println!("import_task_annotations: Creating annotation {} for task {}", annotation_id, task_id);
    println!("import_task_annotations: Will create {} image_annotations", annotations.len());

    // Create single annotation for this task
    sqlx::query!(
        r#"
        INSERT INTO annotations (id, task_id, metadata, annotated_by, annotated_at, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        "#,
        annotation_id,
        task_id,
        serde_json::json!({"imported_from_coco": true, "original_coco_ids": coco_ids}),
        user_id,
        now,
        now,
        now
    )
    .execute(&mut **tx)
    .await?;

    // Create image annotations for each bounding box
    for (coco_annotation, category_id) in annotations {
        let image_annotation_id = Uuid::new_v4();
        
        sqlx::query!(
            r#"
            INSERT INTO image_annotations (id, annotation_id, category_id, bbox, area, iscrowd, image_metadata, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            "#,
            image_annotation_id,
            annotation_id,
            Some(*category_id),
            &coco_annotation.bbox,
            Some(coco_annotation.area as f64),
            coco_annotation.iscrowd == 1,
            serde_json::json!({"imported_from_coco": true, "original_coco_id": coco_annotation.id}),
            now,
            now
        )
        .execute(&mut **tx)
        .await?;
    }

    Ok(annotations.len())
}