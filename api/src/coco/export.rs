use actix_web::{web, HttpResponse, Responder, HttpRequest};
use sqlx::{Pool, Postgres};
use uuid::Uuid;
use chrono::{Datelike, Utc};
use serde::Serialize;

use crate::auth::{JwtManager, Claims};
use super::types::{CocoExport, CocoInfo, CocoLicense, CocoImage, CocoAnnotation, CocoCategory};

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
        licenses: vec![CocoLicense {
            id: 1,
            name: "Unknown License".to_string(),
            url: "https://fast-tag.com/license".to_string(),
        }],
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
    let pretty_json = {
        let mut pretty = serde_json::Serializer::with_formatter(
            Vec::new(),
            serde_json::ser::PrettyFormatter::with_indent(b"  ")
        );
        match coco_export.serialize(&mut pretty) {
            Ok(_) => String::from_utf8_lossy(&pretty.into_inner()).to_string(),
            Err(_) => return HttpResponse::InternalServerError().json("Failed to serialize JSON"),
        }
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
        SELECT id, name, resource_url, created_at, width, height
        FROM tasks
        WHERE project_id = $1
        ORDER BY created_at
        "#,
        project_id
    )
    .fetch_all(pool)
    .await?;

    // Then get latest annotations with categories for each task
    let annotation_rows = sqlx::query!(
        r#"
        WITH latest_annotations AS (
            SELECT 
                id,
                task_id,
                ROW_NUMBER() OVER (PARTITION BY task_id ORDER BY created_at DESC) as rn
            FROM annotations a2
            WHERE task_id IN (SELECT id FROM tasks WHERE project_id = $1)
        )
        SELECT 
            a.id as annotation_id,
            a.task_id,
            ia.bbox,
            ia.area,
            ia.iscrowd,
            iac.coco_id as category_coco_id,
            iac.id as category_id
        FROM latest_annotations la
        JOIN annotations a ON la.id = a.id
        JOIN image_annotations ia ON a.id = ia.annotation_id
        JOIN image_annotation_categories iac ON ia.category_id = iac.id
        WHERE la.rn = 1
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
            .and_then(|url| url.split('/').next_back())
            .unwrap_or(&task.name)
            .to_string();

        images.push(CocoImage {
            id: image_id,
            width: task.width.unwrap_or(0),
            height: task.height.unwrap_or(0),
            file_name,
            license: 1, // Default license ID
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
            let bbox_vec: Vec<f64> = row.bbox.into_iter().collect();
            
            // Calculate area if not provided
            let area = row.area.unwrap_or_else(|| {
                if bbox_vec.len() >= 4 {
                    bbox_vec[2] * bbox_vec[3] // width * height
                } else {
                    0.0
                }
            }) as i32;

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

pub(super) async fn user_has_project_access(pool: &Pool<Postgres>, project_id: Uuid, user_id: Uuid) -> bool {
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

pub(super) fn extract_user_claims(
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