use actix_web::{web, HttpResponse, Responder, HttpRequest};
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Postgres};
use uuid::Uuid;
use chrono::{DateTime, Utc};

use crate::auth::{JwtManager, Claims};

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Annotation {
    pub id: Uuid,
    pub task_id: Uuid,
    pub metadata: serde_json::Value,
    pub annotated_by: Option<Uuid>,
    pub annotated_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct ImageAnnotation {
    pub id: Uuid,
    pub annotation_id: Uuid,
    pub category_id: Option<Uuid>,
    pub bbox: Vec<f64>,
    pub area: Option<f64>,
    pub iscrowd: bool,
    pub image_metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AnnotationWithCategory {
    pub id: Uuid,
    pub task_id: Uuid,
    pub metadata: serde_json::Value,
    pub annotated_by: Option<Uuid>,
    pub annotated_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub annotation_id: Uuid,
    pub category_id: Option<Uuid>,
    pub bbox: Vec<f64>,
    pub area: Option<f64>,
    pub iscrowd: bool,
    pub image_metadata: serde_json::Value,
    pub category_name: String,
    pub category_color: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BoundingBox {
    pub category_id: Uuid,
    pub bbox: Vec<f64>, // [x, y, width, height]
    pub area: Option<f64>,
    pub iscrowd: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateAnnotationRequest {
    pub bboxes: Vec<BoundingBox>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateAnnotationRequest {
    pub bboxes: Vec<BoundingBox>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
pub struct AnnotationResponse {
    pub annotations: Vec<AnnotationWithCategory>,
}

#[derive(Debug, Serialize)]
pub struct AnnotationsListResponse {
    pub annotations: Vec<AnnotationWithCategory>,
}

pub async fn create_annotation(
    req: HttpRequest,
    path: web::Path<(String, String)>,
    payload: web::Json<CreateAnnotationRequest>,
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

    let (project_id_str, task_id_str) = path.into_inner();
    let project_id = match Uuid::parse_str(&project_id_str) {
        Ok(id) => id,
        Err(_) => return HttpResponse::BadRequest().json("Invalid project ID"),
    };

    let task_id = match Uuid::parse_str(&task_id_str) {
        Ok(id) => id,
        Err(_) => return HttpResponse::BadRequest().json("Invalid task ID"),
    };

    // Validate bboxes format
    if payload.bboxes.is_empty() {
        return HttpResponse::BadRequest().json("At least one bounding box is required");
    }

    for bbox in &payload.bboxes {
        if bbox.bbox.len() != 4 {
            return HttpResponse::BadRequest().json("Each bbox must have exactly 4 values [x, y, width, height]");
        }

        for &value in &bbox.bbox {
            if value < 0.0 {
                return HttpResponse::BadRequest().json("bbox values must be non-negative");
            }
        }
    }

    // Check if user has access to this project
    if !user_has_project_access(&pool, project_id, user_id).await {
        return HttpResponse::NotFound().json("Project not found or access denied");
    }

    // Verify task belongs to the project
    if !task_belongs_to_project(&pool, task_id, project_id).await {
        return HttpResponse::BadRequest().json("Task does not belong to the specified project");
    }

    // Verify all categories belong to the project
    for bbox in &payload.bboxes {
        if !category_belongs_to_project(&pool, bbox.category_id, project_id).await {
            return HttpResponse::BadRequest().json("One or more categories do not belong to the specified project");
        }
    }

    // Create annotation with multiple bounding boxes
    match create_annotation_in_db(
        &pool,
        task_id,
        &payload.bboxes,
        payload.metadata.as_ref().unwrap_or(&serde_json::json!({})),
        user_id,
    ).await {
        Ok(annotations) => HttpResponse::Created().json(AnnotationResponse { 
            annotations 
        }),
        Err(_) => HttpResponse::InternalServerError().json("Failed to create annotation"),
    }
}

pub async fn list_annotations(
    req: HttpRequest,
    path: web::Path<(String, String)>,
    query: web::Query<std::collections::HashMap<String, String>>,
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

    let (project_id_str, task_id_str) = path.into_inner();
    let project_id = match Uuid::parse_str(&project_id_str) {
        Ok(id) => id,
        Err(_) => return HttpResponse::BadRequest().json("Invalid project ID"),
    };

    let task_id = match Uuid::parse_str(&task_id_str) {
        Ok(id) => id,
        Err(_) => return HttpResponse::BadRequest().json("Invalid task ID"),
    };

    // Check if user has access to this project
    if !user_has_project_access(&pool, project_id, user_id).await {
        return HttpResponse::NotFound().json("Project not found or access denied");
    }

    // Verify task belongs to the project
    if !task_belongs_to_project(&pool, task_id, project_id).await {
        return HttpResponse::BadRequest().json("Task does not belong to the specified project");
    }

    // Check if latest_only flag is set
    let latest_only = query.get("latest_only").map(|v| v == "true").unwrap_or(false);

    // Get task's annotations
    match get_task_annotations(&pool, task_id, latest_only).await {
        Ok(annotations) => HttpResponse::Ok().json(AnnotationsListResponse { annotations }),
        Err(_) => HttpResponse::InternalServerError().json("Failed to fetch annotations"),
    }
}

pub async fn get_annotation(
    req: HttpRequest,
    path: web::Path<(String, String, String)>,
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

    let (project_id_str, task_id_str, annotation_id_str) = path.into_inner();
    let project_id = match Uuid::parse_str(&project_id_str) {
        Ok(id) => id,
        Err(_) => return HttpResponse::BadRequest().json("Invalid project ID"),
    };

    let task_id = match Uuid::parse_str(&task_id_str) {
        Ok(id) => id,
        Err(_) => return HttpResponse::BadRequest().json("Invalid task ID"),
    };

    let annotation_id = match Uuid::parse_str(&annotation_id_str) {
        Ok(id) => id,
        Err(_) => return HttpResponse::BadRequest().json("Invalid annotation ID"),
    };

    // Check if user has access to this project
    if !user_has_project_access(&pool, project_id, user_id).await {
        return HttpResponse::NotFound().json("Project not found or access denied");
    }

    // Verify task belongs to the project
    if !task_belongs_to_project(&pool, task_id, project_id).await {
        return HttpResponse::BadRequest().json("Task does not belong to the specified project");
    }

    // Get annotation
    match get_annotation_by_id(&pool, annotation_id, task_id).await {
        Ok(Some(annotations)) => HttpResponse::Ok().json(AnnotationResponse { annotations }),
        Ok(None) => HttpResponse::NotFound().json("Annotation not found"),
        Err(_) => HttpResponse::InternalServerError().json("Failed to fetch annotation"),
    }
}

pub async fn update_annotation(
    req: HttpRequest,
    path: web::Path<(String, String, String)>,
    payload: web::Json<UpdateAnnotationRequest>,
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

    let (project_id_str, task_id_str, annotation_id_str) = path.into_inner();
    let project_id = match Uuid::parse_str(&project_id_str) {
        Ok(id) => id,
        Err(_) => return HttpResponse::BadRequest().json("Invalid project ID"),
    };

    let task_id = match Uuid::parse_str(&task_id_str) {
        Ok(id) => id,
        Err(_) => return HttpResponse::BadRequest().json("Invalid task ID"),
    };

    let annotation_id = match Uuid::parse_str(&annotation_id_str) {
        Ok(id) => id,
        Err(_) => return HttpResponse::BadRequest().json("Invalid annotation ID"),
    };

    // Validate bboxes format
    if payload.bboxes.is_empty() {
        return HttpResponse::BadRequest().json("At least one bounding box is required");
    }

    for bbox in &payload.bboxes {
        if bbox.bbox.len() != 4 {
            return HttpResponse::BadRequest().json("Each bbox must have exactly 4 values [x, y, width, height]");
        }

        for &value in &bbox.bbox {
            if value < 0.0 {
                return HttpResponse::BadRequest().json("bbox values must be non-negative");
            }
        }
    }

    // Check if user has access to this project
    if !user_has_project_access(&pool, project_id, user_id).await {
        return HttpResponse::NotFound().json("Project not found or access denied");
    }

    // Verify task belongs to the project
    if !task_belongs_to_project(&pool, task_id, project_id).await {
        return HttpResponse::BadRequest().json("Task does not belong to the specified project");
    }

    // Verify all categories belong to the project
    for bbox in &payload.bboxes {
        if !category_belongs_to_project(&pool, bbox.category_id, project_id).await {
            return HttpResponse::BadRequest().json("One or more categories do not belong to the specified project");
        }
    }

    // Update annotation with multiple bounding boxes
    match update_annotation_in_db(
        &pool,
        annotation_id,
        task_id,
        &payload.bboxes,
        payload.metadata.as_ref().unwrap_or(&serde_json::json!({})),
    ).await {
        Ok(Some(annotations)) => HttpResponse::Ok().json(AnnotationResponse { annotations }),
        Ok(None) => HttpResponse::NotFound().json("Annotation not found"),
        Err(_) => HttpResponse::InternalServerError().json("Failed to update annotation"),
    }
}

pub async fn delete_annotation(
    req: HttpRequest,
    path: web::Path<(String, String, String)>,
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

    let (project_id_str, task_id_str, annotation_id_str) = path.into_inner();
    let project_id = match Uuid::parse_str(&project_id_str) {
        Ok(id) => id,
        Err(_) => return HttpResponse::BadRequest().json("Invalid project ID"),
    };

    let task_id = match Uuid::parse_str(&task_id_str) {
        Ok(id) => id,
        Err(_) => return HttpResponse::BadRequest().json("Invalid task ID"),
    };

    let annotation_id = match Uuid::parse_str(&annotation_id_str) {
        Ok(id) => id,
        Err(_) => return HttpResponse::BadRequest().json("Invalid annotation ID"),
    };

    // Check if user has access to this project
    if !user_has_project_access(&pool, project_id, user_id).await {
        return HttpResponse::NotFound().json("Project not found or access denied");
    }

    // Verify task belongs to the project
    if !task_belongs_to_project(&pool, task_id, project_id).await {
        return HttpResponse::BadRequest().json("Task does not belong to the specified project");
    }

    // Delete annotation
    match delete_annotation_from_db(&pool, annotation_id, task_id).await {
        Ok(true) => HttpResponse::NoContent().finish(),
        Ok(false) => HttpResponse::NotFound().json("Annotation not found"),
        Err(_) => HttpResponse::InternalServerError().json("Failed to delete annotation"),
    }
}

pub async fn create_annotation_in_db(
    pool: &Pool<Postgres>,
    task_id: Uuid,
    bboxes: &[BoundingBox],
    metadata: &serde_json::Value,
    annotated_by: Uuid,
) -> Result<Vec<AnnotationWithCategory>, sqlx::Error> {
    if bboxes.is_empty() {
        return Ok(Vec::new());
    }

    let annotation_id = Uuid::new_v4();
    let now = Utc::now();

    // Create single annotation (always creates new annotation to preserve history)
    let annotation = sqlx::query_as::<_, Annotation>(
        r#"
        INSERT INTO annotations (id, task_id, metadata, annotated_by, annotated_at, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        RETURNING id, task_id, metadata, annotated_by, annotated_at, created_at, updated_at
        "#
    )
    .bind(annotation_id)
    .bind(task_id)
    .bind(metadata)
    .bind(annotated_by)
    .bind(now)
    .bind(now)
    .bind(now)
    .fetch_one(pool)
    .await?;

    let mut result = Vec::new();

    // Create multiple image annotations for each bounding box
    for bbox in bboxes {
        let image_annotation_id = Uuid::new_v4();
        
        // Calculate area if not provided
        let calculated_area = bbox.area.unwrap_or_else(|| {
            if bbox.bbox.len() >= 4 {
                let width = bbox.bbox[2];
                let height = bbox.bbox[3];
                width * height
            } else {
                0.0
            }
        });

        let image_annotation = sqlx::query_as::<_, ImageAnnotation>(
            r#"
            INSERT INTO image_annotations (id, annotation_id, category_id, bbox, area, iscrowd, image_metadata, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            RETURNING id, annotation_id, category_id, bbox, area, iscrowd, image_metadata, created_at, updated_at
            "#
        )
        .bind(image_annotation_id)
        .bind(annotation_id)
        .bind(Some(bbox.category_id))
        .bind(&bbox.bbox)
        .bind(Some(calculated_area))
        .bind(bbox.iscrowd.unwrap_or(false))
        .bind(serde_json::json!({}))
        .bind(now)
        .bind(now)
        .fetch_one(pool)
        .await?;

        // Get category info from image_annotation_categories
        let category = sqlx::query!(
            "SELECT name, color FROM image_annotation_categories WHERE id = $1",
            bbox.category_id
        )
        .fetch_one(pool)
        .await?;

        result.push(AnnotationWithCategory {
            id: image_annotation.id,
            task_id: annotation.task_id,
            metadata: annotation.metadata.clone(),
            annotated_by: annotation.annotated_by,
            annotated_at: annotation.annotated_at,
            created_at: annotation.created_at,
            updated_at: annotation.updated_at,
            annotation_id: annotation.id,
            category_id: image_annotation.category_id,
            bbox: image_annotation.bbox,
            area: image_annotation.area,
            iscrowd: image_annotation.iscrowd,
            image_metadata: image_annotation.image_metadata,
            category_name: category.name,
            category_color: category.color,
        });
    }

    Ok(result)
}

async fn get_task_annotations(
    pool: &Pool<Postgres>,
    task_id: Uuid,
    latest_only: bool,
) -> Result<Vec<AnnotationWithCategory>, sqlx::Error> {
    let mut where_clause = "WHERE a.task_id = $1".to_string();
    
    if latest_only {
        where_clause.push_str(" AND a.id = (SELECT id FROM annotations WHERE task_id = $1 ORDER BY created_at DESC LIMIT 1)");
    }
    
    let query = format!(
        r#"
        SELECT 
            a.id, a.task_id, a.metadata, a.annotated_by, a.annotated_at, a.created_at, a.updated_at,
            ia.id as image_id, ia.annotation_id, ia.category_id, ia.bbox, ia.area, ia.iscrowd, ia.image_metadata, ia.created_at as image_created_at, ia.updated_at as image_updated_at,
            COALESCE(iac.name, 'Unknown') as category_name,
            iac.color as category_color
        FROM annotations a
        JOIN image_annotations ia ON a.id = ia.annotation_id
        LEFT JOIN image_annotation_categories iac ON ia.category_id = iac.id
        {}
        ORDER BY a.created_at ASC
        "#,
        where_clause
    );

    let rows = sqlx::query(&query)
        .bind(task_id)
        .fetch_all(pool)
        .await?;

    let mut result = Vec::new();
    for row in rows {
        use sqlx::Row;
        
        let annotation = Annotation {
            id: row.get("id"),
            task_id: row.get("task_id"),
            metadata: row.get::<Option<serde_json::Value>, _>("metadata").unwrap_or_else(|| serde_json::json!({})),
            annotated_by: row.get("annotated_by"),
            annotated_at: row.get::<Option<chrono::DateTime<chrono::Utc>>, _>("annotated_at").unwrap_or_else(|| row.get("created_at")),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        };

        let image_annotation = ImageAnnotation {
            id: row.get("image_id"),
            annotation_id: row.get("annotation_id"),
            category_id: row.get("category_id"),
            bbox: row.get("bbox"),
            area: row.get("area"),
            iscrowd: row.get::<Option<bool>, _>("iscrowd").unwrap_or(false),
            image_metadata: row.get::<Option<serde_json::Value>, _>("image_metadata").unwrap_or_else(|| serde_json::json!({})),
            created_at: row.get::<Option<chrono::DateTime<chrono::Utc>>, _>("image_created_at").unwrap_or_else(|| row.get("created_at")),
            updated_at: row.get::<Option<chrono::DateTime<chrono::Utc>>, _>("image_updated_at").unwrap_or_else(|| row.get("updated_at")),
        };

        result.push(AnnotationWithCategory {
            id: image_annotation.id,
            task_id: annotation.task_id,
            metadata: annotation.metadata,
            annotated_by: annotation.annotated_by,
            annotated_at: annotation.annotated_at,
            created_at: annotation.created_at,
            updated_at: annotation.updated_at,
            annotation_id: annotation.id,
            category_id: image_annotation.category_id,
            bbox: image_annotation.bbox,
            area: image_annotation.area,
            iscrowd: image_annotation.iscrowd,
            image_metadata: image_annotation.image_metadata,
            category_name: row.get::<Option<String>, _>("category_name").unwrap_or_else(|| "Unknown".to_string()),
            category_color: row.get("category_color"),
        });
    }

    Ok(result)
}

async fn get_annotation_by_id(
    pool: &Pool<Postgres>,
    annotation_id: Uuid,
    task_id: Uuid,
) -> Result<Option<Vec<AnnotationWithCategory>>, sqlx::Error> {
    let rows = sqlx::query!(
        r#"
        SELECT 
            a.id, a.task_id, a.metadata, a.annotated_by, a.annotated_at, a.created_at, a.updated_at,
            ia.id as image_id, ia.annotation_id, ia.category_id, ia.bbox, ia.area, ia.iscrowd, ia.image_metadata, ia.created_at as image_created_at, ia.updated_at as image_updated_at,
            COALESCE(iac.name, 'Unknown') as category_name,
            iac.color as category_color
        FROM annotations a
        JOIN image_annotations ia ON a.id = ia.annotation_id
        LEFT JOIN image_annotation_categories iac ON ia.category_id = iac.id
        WHERE a.id = $1 AND a.task_id = $2
        ORDER BY ia.created_at ASC
        "#,
        annotation_id,
        task_id
    )
    .fetch_all(pool)
    .await?;

    if rows.is_empty() {
        return Ok(None);
    }

    let mut result = Vec::new();
    for row in rows {
        let annotation = Annotation {
            id: row.id,
            task_id: row.task_id,
            metadata: row.metadata.unwrap_or_else(|| serde_json::json!({})),
            annotated_by: row.annotated_by,
            annotated_at: row.annotated_at.unwrap_or_else(|| row.created_at.unwrap()),
            created_at: row.created_at.unwrap(),
            updated_at: row.updated_at.unwrap(),
        };

        let image_annotation = ImageAnnotation {
            id: row.image_id,
            annotation_id: row.annotation_id,
            category_id: row.category_id,
            bbox: row.bbox,
            area: row.area,
            iscrowd: row.iscrowd.unwrap_or(false),
            image_metadata: row.image_metadata.unwrap_or_else(|| serde_json::json!({})),
            created_at: row.image_created_at.unwrap_or_else(|| row.created_at.unwrap()),
            updated_at: row.image_updated_at.unwrap_or_else(|| row.updated_at.unwrap()),
        };

        result.push(AnnotationWithCategory {
            id: image_annotation.id,
            task_id: annotation.task_id,
            metadata: annotation.metadata,
            annotated_by: annotation.annotated_by,
            annotated_at: annotation.annotated_at,
            created_at: annotation.created_at,
            updated_at: annotation.updated_at,
            annotation_id: annotation.id,
            category_id: image_annotation.category_id,
            bbox: image_annotation.bbox,
            area: image_annotation.area,
            iscrowd: image_annotation.iscrowd,
            image_metadata: image_annotation.image_metadata,
            category_name: row.category_name.unwrap_or("Unknown".to_string()),
            category_color: row.category_color,
        });
    }

    Ok(Some(result))
}

async fn update_annotation_in_db(
    pool: &Pool<Postgres>,
    annotation_id: Uuid,
    task_id: Uuid,
    bboxes: &[BoundingBox],
    metadata: &serde_json::Value,
) -> Result<Option<Vec<AnnotationWithCategory>>, sqlx::Error> {
    let now = Utc::now();

    // Update annotation
    let annotation = match sqlx::query_as::<_, Annotation>(
        r#"
        UPDATE annotations 
        SET metadata = $1, updated_at = $2
        WHERE id = $3 AND task_id = $4
        RETURNING id, task_id, metadata, annotated_by, annotated_at, created_at, updated_at
        "#
    )
    .bind(metadata)
    .bind(now)
    .bind(annotation_id)
    .bind(task_id)
    .fetch_optional(pool)
    .await? {
        Some(annotation) => annotation,
        None => return Ok(None),
    };

    // Delete existing image annotations
    sqlx::query!("DELETE FROM image_annotations WHERE annotation_id = $1", annotation_id)
        .execute(pool)
        .await?;

    let mut result = Vec::new();

    // Create new image annotations for each bounding box
    for bbox in bboxes {
        let image_annotation_id = Uuid::new_v4();
        
        // Calculate area if not provided
        let calculated_area = bbox.area.unwrap_or_else(|| {
            if bbox.bbox.len() >= 4 {
                let width = bbox.bbox[2];
                let height = bbox.bbox[3];
                width * height
            } else {
                0.0
            }
        });

        let image_annotation = sqlx::query_as::<_, ImageAnnotation>(
            r#"
            INSERT INTO image_annotations (id, annotation_id, category_id, bbox, area, iscrowd, image_metadata, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            RETURNING id, annotation_id, category_id, bbox, area, iscrowd, image_metadata, created_at, updated_at
            "#
        )
        .bind(image_annotation_id)
        .bind(annotation_id)
        .bind(Some(bbox.category_id))
        .bind(&bbox.bbox)
        .bind(Some(calculated_area))
        .bind(bbox.iscrowd.unwrap_or(false))
        .bind(serde_json::json!({}))
        .bind(now)
        .bind(now)
        .fetch_one(pool)
        .await?;

        // Get category info from image_annotation_categories
        let category = sqlx::query!(
            "SELECT name, color FROM image_annotation_categories WHERE id = $1",
            bbox.category_id
        )
        .fetch_one(pool)
        .await?;

        result.push(AnnotationWithCategory {
            id: image_annotation.id,
            task_id: annotation.task_id,
            metadata: annotation.metadata.clone(),
            annotated_by: annotation.annotated_by,
            annotated_at: annotation.annotated_at,
            created_at: annotation.created_at,
            updated_at: annotation.updated_at,
            annotation_id: annotation.id,
            category_id: image_annotation.category_id,
            bbox: image_annotation.bbox,
            area: image_annotation.area,
            iscrowd: image_annotation.iscrowd,
            image_metadata: image_annotation.image_metadata,
            category_name: category.name,
            category_color: category.color,
        });
    }

    Ok(Some(result))
}

async fn delete_annotation_from_db(
    pool: &Pool<Postgres>,
    annotation_id: Uuid,
    task_id: Uuid,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query("DELETE FROM annotations WHERE id = $1 AND task_id = $2")
        .bind(annotation_id)
        .bind(task_id)
        .execute(pool)
        .await?;

    Ok(result.rows_affected() > 0)
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

async fn task_belongs_to_project(pool: &Pool<Postgres>, task_id: Uuid, project_id: Uuid) -> bool {
    sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM tasks WHERE id = $1 AND project_id = $2)"
    )
    .bind(task_id)
    .bind(project_id)
    .fetch_one(pool)
    .await
    .unwrap_or(false)
}

async fn category_belongs_to_project(pool: &Pool<Postgres>, category_id: Uuid, project_id: Uuid) -> bool {
    sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM image_annotation_categories WHERE id = $1 AND project_id = $2)"
    )
    .bind(category_id)
    .bind(project_id)
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
    async fn test_create_annotation_success() {
        let pool = test_utils::setup_test_db().await;
        let user = test_utils::create_test_user_with_details(&pool).await;
        let oauth_config = create_test_oauth_config();
        let token = create_auth_token(&oauth_config, &user);
        let auth_storage = AuthStorage::new(pool.clone());

        // Create test project, category, and task
        let project = crate::projects::create_project_in_db(&pool, "Test Project", Some("Description"), None, user.id).await.unwrap();
        let category = crate::image_annotation_categories::create_image_annotation_category_in_db(&pool, project.id, "person", None, Some("human"), Some("#FF0000"), Some(1)).await.unwrap();
        let task = crate::tasks::create_task_in_db(&pool, project.id, "Test Task", Some("test.jpg")).await.unwrap();

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(pool.clone()))
                .app_data(web::Data::new(oauth_config))
                .app_data(web::Data::new(auth_storage))
                .route("/projects/{project_id}/tasks/{task_id}/annotations", web::post().to(create_annotation))
        ).await;

        let create_request = CreateAnnotationRequest {
            bboxes: vec![BoundingBox {
                category_id: category.id,
                bbox: vec![100.0, 50.0, 200.0, 150.0], // [x, y, width, height]
                area: Some(30000.0),
                iscrowd: Some(false),
            }],
            metadata: Some(serde_json::json!({"confidence": 0.95})),
        };

        let req = test::TestRequest::post()
            .uri(&format!("/projects/{}/tasks/{}/annotations", project.id, task.id))
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .set_json(create_request)
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 201);

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["annotations"].as_array().unwrap().len(), 1);
        assert_eq!(body["annotations"][0]["category_name"], "person");
        assert_eq!(body["annotations"][0]["bbox"], serde_json::json!([100.0, 50.0, 200.0, 150.0]));
        assert_eq!(body["annotations"][0]["area"], 30000.0);
        assert_eq!(body["annotations"][0]["iscrowd"], false);
    }

    #[actix_web::test]
    #[serial]
    async fn test_create_annotation_invalid_bbox() {
        let pool = test_utils::setup_test_db().await;
        let user = test_utils::create_test_user_with_details(&pool).await;
        let oauth_config = create_test_oauth_config();
        let token = create_auth_token(&oauth_config, &user);
        let auth_storage = AuthStorage::new(pool.clone());

        let project = crate::projects::create_project_in_db(&pool, "Test Project", Some("Description"), None, user.id).await.unwrap();
        let category = crate::image_annotation_categories::create_image_annotation_category_in_db(&pool, project.id, "person", None, Some("human"), Some("#FF0000"), Some(1)).await.unwrap();
        let task = crate::tasks::create_task_in_db(&pool, project.id, "Test Task", Some("test.jpg")).await.unwrap();

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(pool.clone()))
                .app_data(web::Data::new(oauth_config))
                .app_data(web::Data::new(auth_storage))
                .route("/projects/{project_id}/tasks/{task_id}/annotations", web::post().to(create_annotation))
        ).await;

        let create_request = CreateAnnotationRequest {
            bboxes: vec![BoundingBox {
                category_id: category.id,
                bbox: vec![100.0, 50.0, 200.0], // Invalid: only 3 values
                area: None,
                iscrowd: None,
            }],
            metadata: None,
        };

        let req = test::TestRequest::post()
            .uri(&format!("/projects/{}/tasks/{}/annotations", project.id, task.id))
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .set_json(create_request)
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 400);
    }

    #[actix_web::test]
    #[serial]
    async fn test_create_annotation_negative_bbox() {
        let pool = test_utils::setup_test_db().await;
        let user = test_utils::create_test_user_with_details(&pool).await;
        let oauth_config = create_test_oauth_config();
        let token = create_auth_token(&oauth_config, &user);
        let auth_storage = AuthStorage::new(pool.clone());

        let project = crate::projects::create_project_in_db(&pool, "Test Project", Some("Description"), None, user.id).await.unwrap();
        let category = crate::image_annotation_categories::create_image_annotation_category_in_db(&pool, project.id, "person", None, Some("human"), Some("#FF0000"), Some(1)).await.unwrap();
        let task = crate::tasks::create_task_in_db(&pool, project.id, "Test Task", Some("test.jpg")).await.unwrap();

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(pool.clone()))
                .app_data(web::Data::new(oauth_config))
                .app_data(web::Data::new(auth_storage))
                .route("/projects/{project_id}/tasks/{task_id}/annotations", web::post().to(create_annotation))
        ).await;

        let create_request = CreateAnnotationRequest {
            bboxes: vec![BoundingBox {
                category_id: category.id,
                bbox: vec![-10.0, 50.0, 200.0, 150.0], // Invalid: negative value
                area: None,
                iscrowd: None,
            }],
            metadata: None,
        };

        let req = test::TestRequest::post()
            .uri(&format!("/projects/{}/tasks/{}/annotations", project.id, task.id))
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .set_json(create_request)
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 400);
    }

    #[actix_web::test]
    #[serial]
    async fn test_list_annotations_success() {
        let pool = test_utils::setup_test_db().await;
        let user = test_utils::create_test_user_with_details(&pool).await;
        let oauth_config = create_test_oauth_config();
        let token = create_auth_token(&oauth_config, &user);
        let auth_storage = AuthStorage::new(pool.clone());

        let project = crate::projects::create_project_in_db(&pool, "Test Project", Some("Description"), None, user.id).await.unwrap();
        let category = crate::image_annotation_categories::create_image_annotation_category_in_db(&pool, project.id, "person", None, Some("human"), Some("#FF0000"), Some(1)).await.unwrap();
        let task = crate::tasks::create_task_in_db(&pool, project.id, "Test Task", Some("test.jpg")).await.unwrap();

        // Create test annotations
        let bbox1 = BoundingBox { category_id: category.id, bbox: vec![100.0, 50.0, 200.0, 150.0], area: Some(30000.0), iscrowd: Some(false) };
        let bbox2 = BoundingBox { category_id: category.id, bbox: vec![300.0, 100.0, 150.0, 100.0], area: Some(15000.0), iscrowd: Some(false) };
        create_annotation_in_db(&pool, task.id, &[bbox1], &serde_json::json!({}), user.id).await.unwrap();
        create_annotation_in_db(&pool, task.id, &[bbox2], &serde_json::json!({}), user.id).await.unwrap();

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(pool.clone()))
                .app_data(web::Data::new(oauth_config))
                .app_data(web::Data::new(auth_storage))
                .route("/projects/{project_id}/tasks/{task_id}/annotations", web::get().to(list_annotations))
        ).await;

        let req = test::TestRequest::get()
            .uri(&format!("/projects/{}/tasks/{}/annotations", project.id, task.id))
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["annotations"].as_array().unwrap().len(), 2);
    }

    #[actix_web::test]
    #[serial]
    async fn test_get_annotation_success() {
        let pool = test_utils::setup_test_db().await;
        let user = test_utils::create_test_user_with_details(&pool).await;
        let oauth_config = create_test_oauth_config();
        let token = create_auth_token(&oauth_config, &user);
        let auth_storage = AuthStorage::new(pool.clone());

        let project = crate::projects::create_project_in_db(&pool, "Test Project", Some("Description"), None, user.id).await.unwrap();
        let category = crate::image_annotation_categories::create_image_annotation_category_in_db(&pool, project.id, "person", None, Some("human"), Some("#FF0000"), Some(1)).await.unwrap();
        let task = crate::tasks::create_task_in_db(&pool, project.id, "Test Task", Some("test.jpg")).await.unwrap();

        let bbox = BoundingBox { category_id: category.id, bbox: vec![100.0, 50.0, 200.0, 150.0], area: Some(30000.0), iscrowd: Some(false) };
        let annotations = create_annotation_in_db(&pool, task.id, &[bbox], &serde_json::json!({}), user.id).await.unwrap();
        let annotation = &annotations[0];

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(pool.clone()))
                .app_data(web::Data::new(oauth_config))
                .app_data(web::Data::new(auth_storage))
                .route("/projects/{project_id}/tasks/{task_id}/annotations/{annotation_id}", web::get().to(get_annotation))
        ).await;

        let req = test::TestRequest::get()
            .uri(&format!("/projects/{}/tasks/{}/annotations/{}", project.id, task.id, annotation.annotation_id))
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["annotations"][0]["category_name"], "person");
        assert_eq!(body["annotations"][0]["bbox"], serde_json::json!([100.0, 50.0, 200.0, 150.0]));
    }

    #[actix_web::test]
    #[serial]
    async fn test_update_annotation_success() {
        let pool = test_utils::setup_test_db().await;
        let user = test_utils::create_test_user_with_details(&pool).await;
        let oauth_config = create_test_oauth_config();
        let token = create_auth_token(&oauth_config, &user);
        let auth_storage = AuthStorage::new(pool.clone());

        let project = crate::projects::create_project_in_db(&pool, "Test Project", Some("Description"), None, user.id).await.unwrap();
        let category = crate::image_annotation_categories::create_image_annotation_category_in_db(&pool, project.id, "person", None, Some("human"), Some("#FF0000"), Some(1)).await.unwrap();
        let task = crate::tasks::create_task_in_db(&pool, project.id, "Test Task", Some("test.jpg")).await.unwrap();

        let bbox = BoundingBox { category_id: category.id, bbox: vec![100.0, 50.0, 200.0, 150.0], area: Some(30000.0), iscrowd: Some(false) };
        let annotations = create_annotation_in_db(&pool, task.id, &[bbox], &serde_json::json!({}), user.id).await.unwrap();
        let annotation = &annotations[0];

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(pool.clone()))
                .app_data(web::Data::new(oauth_config))
                .app_data(web::Data::new(auth_storage))
                .route("/projects/{project_id}/tasks/{task_id}/annotations/{annotation_id}", web::put().to(update_annotation))
        ).await;

        let update_request = UpdateAnnotationRequest {
            bboxes: vec![BoundingBox {
                category_id: category.id,
                bbox: vec![120.0, 60.0, 180.0, 140.0], // Updated bbox
                area: Some(25200.0),
                iscrowd: Some(true),
            }],
            metadata: Some(serde_json::json!({"confidence": 0.85, "updated": true})),
        };

        let req = test::TestRequest::put()
            .uri(&format!("/projects/{}/tasks/{}/annotations/{}", project.id, task.id, annotation.annotation_id))
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .set_json(update_request)
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["annotations"][0]["bbox"], serde_json::json!([120.0, 60.0, 180.0, 140.0]));
        assert_eq!(body["annotations"][0]["area"], 25200.0);
        assert_eq!(body["annotations"][0]["iscrowd"], true);
    }

    #[actix_web::test]
    #[serial]
    async fn test_delete_annotation_success() {
        let pool = test_utils::setup_test_db().await;
        let user = test_utils::create_test_user_with_details(&pool).await;
        let oauth_config = create_test_oauth_config();
        let token = create_auth_token(&oauth_config, &user);
        let auth_storage = AuthStorage::new(pool.clone());

        let project = crate::projects::create_project_in_db(&pool, "Test Project", Some("Description"), None, user.id).await.unwrap();
        let category = crate::image_annotation_categories::create_image_annotation_category_in_db(&pool, project.id, "person", None, Some("human"), Some("#FF0000"), Some(1)).await.unwrap();
        let task = crate::tasks::create_task_in_db(&pool, project.id, "Test Task", Some("test.jpg")).await.unwrap();

        let bbox = BoundingBox { category_id: category.id, bbox: vec![100.0, 50.0, 200.0, 150.0], area: Some(30000.0), iscrowd: Some(false) };
        let annotations = create_annotation_in_db(&pool, task.id, &[bbox], &serde_json::json!({}), user.id).await.unwrap();
        let annotation = &annotations[0];

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(pool.clone()))
                .app_data(web::Data::new(oauth_config))
                .app_data(web::Data::new(auth_storage))
                .route("/projects/{project_id}/tasks/{task_id}/annotations/{annotation_id}", web::delete().to(delete_annotation))
        ).await;

        let req = test::TestRequest::delete()
            .uri(&format!("/projects/{}/tasks/{}/annotations/{}", project.id, task.id, annotation.annotation_id))
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 204);
    }

    #[actix_web::test]
    #[serial]
    async fn test_create_annotation_with_wrong_category() {
        let pool = test_utils::setup_test_db().await;
        let user = test_utils::create_test_user_with_details(&pool).await;
        let oauth_config = create_test_oauth_config();
        let token = create_auth_token(&oauth_config, &user);
        let auth_storage = AuthStorage::new(pool.clone());

        let project1 = crate::projects::create_project_in_db(&pool, "Test Project 1", Some("Description"), None, user.id).await.unwrap();
        let project2 = crate::projects::create_project_in_db(&pool, "Test Project 2", Some("Description"), None, user.id).await.unwrap();
        
        let category2 = crate::image_annotation_categories::create_image_annotation_category_in_db(&pool, project2.id, "person", None, Some("human"), Some("#FF0000"), Some(1)).await.unwrap();
        let task1 = crate::tasks::create_task_in_db(&pool, project1.id, "Test Task", Some("test.jpg")).await.unwrap();

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(pool.clone()))
                .app_data(web::Data::new(oauth_config))
                .app_data(web::Data::new(auth_storage))
                .route("/projects/{project_id}/tasks/{task_id}/annotations", web::post().to(create_annotation))
        ).await;

        let create_request = CreateAnnotationRequest {
            bboxes: vec![BoundingBox {
                category_id: category2.id, // Category from different project
                bbox: vec![100.0, 50.0, 200.0, 150.0],
                area: None,
                iscrowd: None,
            }],
            metadata: None,
        };

        let req = test::TestRequest::post()
            .uri(&format!("/projects/{}/tasks/{}/annotations", project1.id, task1.id))
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .set_json(create_request)
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 400);
    }

    #[actix_web::test]
    #[serial]
    async fn test_create_annotation_unauthorized() {
        let pool = test_utils::setup_test_db().await;
        let user = test_utils::create_test_user_with_details(&pool).await;
        let oauth_config = create_test_oauth_config();
        let auth_storage = AuthStorage::new(pool.clone());

        let project = crate::projects::create_project_in_db(&pool, "Test Project", Some("Description"), None, user.id).await.unwrap();
        let category = crate::image_annotation_categories::create_image_annotation_category_in_db(&pool, project.id, "person", None, Some("human"), Some("#FF0000"), Some(1)).await.unwrap();
        let task = crate::tasks::create_task_in_db(&pool, project.id, "Test Task", Some("test.jpg")).await.unwrap();

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(pool.clone()))
                .app_data(web::Data::new(oauth_config))
                .app_data(web::Data::new(auth_storage))
                .route("/projects/{project_id}/tasks/{task_id}/annotations", web::post().to(create_annotation))
        ).await;

        let create_request = CreateAnnotationRequest {
            bboxes: vec![BoundingBox {
                category_id: category.id,
                bbox: vec![100.0, 50.0, 200.0, 150.0],
                area: None,
                iscrowd: None,
            }],
            metadata: None,
        };

        let req = test::TestRequest::post()
            .uri(&format!("/projects/{}/tasks/{}/annotations", project.id, task.id))
            .set_json(create_request)
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 401);
    }
}