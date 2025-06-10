use actix_web::{web, HttpResponse, Responder, HttpRequest};
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Postgres};
use uuid::Uuid;
use chrono::{DateTime, Utc};

use crate::auth::{JwtManager, Claims};
use crate::storage::factory::create_storage_provider_from_project;

#[cfg(test)]
mod tests;

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Task {
    pub id: Uuid,
    pub project_id: Uuid,
    pub name: String,
    pub resource_url: Option<String>,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct CreateTaskRequest {
    pub name: String,
    pub resource_url: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateTaskRequest {
    pub name: String,
    pub resource_url: Option<String>,
    pub status: String,
}

#[derive(Debug, Serialize)]
pub struct TaskResponse {
    pub task: Task,
    pub resolved_resource_url: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct TaskWithResolvedUrl {
    #[serde(flatten)]
    pub task: Task,
    pub resolved_resource_url: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct TasksListResponse {
    pub tasks: Vec<TaskWithResolvedUrl>,
}

pub async fn create_task(
    req: HttpRequest,
    path: web::Path<String>,
    payload: web::Json<CreateTaskRequest>,
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

    // Validate input
    if payload.name.trim().is_empty() {
        return HttpResponse::BadRequest().json("Task name cannot be empty");
    }

    if payload.name.len() > 255 {
        return HttpResponse::BadRequest().json("Task name too long (max 255 characters)");
    }

    // Check if user has access to this project
    if !user_has_project_access(&pool, project_id, user_id).await {
        return HttpResponse::NotFound().json("Project not found or access denied");
    }

    // Create task
    match create_task_in_db(&pool, project_id, &payload.name, payload.resource_url.as_deref()).await {
        Ok(task) => {
            let resolved_url = if let Some(ref url) = task.resource_url {
                resolve_storage_url(&pool, project_id, url).await
            } else {
                None
            };
            HttpResponse::Created().json(TaskResponse { 
                task,
                resolved_resource_url: resolved_url,
            })
        },
        Err(_) => HttpResponse::InternalServerError().json("Failed to create task"),
    }
}

pub async fn list_tasks(
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

    // Get project tasks
    match get_project_tasks(&pool, project_id).await {
        Ok(tasks) => {
            let mut tasks_with_urls = Vec::new();
            for task in tasks {
                let resolved_url = if let Some(ref url) = task.resource_url {
                    resolve_storage_url(&pool, project_id, url).await
                } else {
                    None
                };
                tasks_with_urls.push(TaskWithResolvedUrl {
                    task,
                    resolved_resource_url: resolved_url,
                });
            }
            HttpResponse::Ok().json(TasksListResponse { tasks: tasks_with_urls })
        },
        Err(_) => HttpResponse::InternalServerError().json("Failed to fetch tasks"),
    }
}

pub async fn get_task(
    req: HttpRequest,
    path: web::Path<(String, String)>,
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

    // Get task
    match get_task_by_id(&pool, task_id, project_id).await {
        Ok(Some(task)) => {
            let resolved_url = if let Some(ref url) = task.resource_url {
                resolve_storage_url(&pool, project_id, url).await
            } else {
                None
            };
            HttpResponse::Ok().json(TaskResponse { 
                task,
                resolved_resource_url: resolved_url,
            })
        },
        Ok(None) => HttpResponse::NotFound().json("Task not found"),
        Err(_) => HttpResponse::InternalServerError().json("Failed to fetch task"),
    }
}

pub async fn update_task(
    req: HttpRequest,
    path: web::Path<(String, String)>,
    payload: web::Json<UpdateTaskRequest>,
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

    // Validate input
    if payload.name.trim().is_empty() {
        return HttpResponse::BadRequest().json("Task name cannot be empty");
    }

    if payload.name.len() > 255 {
        return HttpResponse::BadRequest().json("Task name too long (max 255 characters)");
    }

    // Validate status
    if !["pending", "in_progress", "completed", "cancelled"].contains(&payload.status.as_str()) {
        return HttpResponse::BadRequest().json("Invalid status");
    }

    // Check if user has access to this project
    if !user_has_project_access(&pool, project_id, user_id).await {
        return HttpResponse::NotFound().json("Project not found or access denied");
    }

    // Update task
    match update_task_in_db(&pool, task_id, project_id, &payload.name, payload.resource_url.as_deref(), &payload.status).await {
        Ok(Some(task)) => {
            let resolved_url = if let Some(ref url) = task.resource_url {
                resolve_storage_url(&pool, project_id, url).await
            } else {
                None
            };
            HttpResponse::Ok().json(TaskResponse { 
                task,
                resolved_resource_url: resolved_url,
            })
        },
        Ok(None) => HttpResponse::NotFound().json("Task not found"),
        Err(_) => HttpResponse::InternalServerError().json("Failed to update task"),
    }
}

pub async fn delete_task(
    req: HttpRequest,
    path: web::Path<(String, String)>,
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

    // Delete task
    match delete_task_from_db(&pool, task_id, project_id).await {
        Ok(true) => HttpResponse::NoContent().finish(),
        Ok(false) => HttpResponse::NotFound().json("Task not found"),
        Err(_) => HttpResponse::InternalServerError().json("Failed to delete task"),
    }
}

pub async fn create_task_in_db(
    pool: &Pool<Postgres>,
    project_id: Uuid,
    name: &str,
    resource_url: Option<&str>,
) -> Result<Task, sqlx::Error> {
    let task_id = Uuid::new_v4();
    let now = Utc::now();

    sqlx::query_as::<_, Task>(
        r#"
        INSERT INTO tasks (id, project_id, name, resource_url, status, created_at, updated_at)
        VALUES ($1, $2, $3, $4, 'pending', $5, $6)
        RETURNING id, project_id, name, resource_url, status, created_at, updated_at, completed_at
        "#
    )
    .bind(task_id)
    .bind(project_id)
    .bind(name)
    .bind(resource_url)
    .bind(now)
    .bind(now)
    .fetch_one(pool)
    .await
}

async fn get_project_tasks(pool: &Pool<Postgres>, project_id: Uuid) -> Result<Vec<Task>, sqlx::Error> {
    sqlx::query_as::<_, Task>(
        "SELECT id, project_id, name, resource_url, status, created_at, updated_at, completed_at FROM tasks WHERE project_id = $1 ORDER BY created_at DESC"
    )
    .bind(project_id)
    .fetch_all(pool)
    .await
}

async fn get_task_by_id(
    pool: &Pool<Postgres>,
    task_id: Uuid,
    project_id: Uuid,
) -> Result<Option<Task>, sqlx::Error> {
    sqlx::query_as::<_, Task>(
        "SELECT id, project_id, name, resource_url, status, created_at, updated_at, completed_at FROM tasks WHERE id = $1 AND project_id = $2"
    )
    .bind(task_id)
    .bind(project_id)
    .fetch_optional(pool)
    .await
}

async fn update_task_in_db(
    pool: &Pool<Postgres>,
    task_id: Uuid,
    project_id: Uuid,
    name: &str,
    resource_url: Option<&str>,
    status: &str,
) -> Result<Option<Task>, sqlx::Error> {
    let now = Utc::now();
    let completed_at = if status == "completed" { Some(now) } else { None };

    sqlx::query_as::<_, Task>(
        r#"
        UPDATE tasks 
        SET name = $1, resource_url = $2, status = $3, updated_at = $4, completed_at = $5
        WHERE id = $6 AND project_id = $7
        RETURNING id, project_id, name, resource_url, status, created_at, updated_at, completed_at
        "#
    )
    .bind(name)
    .bind(resource_url)
    .bind(status)
    .bind(now)
    .bind(completed_at)
    .bind(task_id)
    .bind(project_id)
    .fetch_optional(pool)
    .await
}

async fn delete_task_from_db(
    pool: &Pool<Postgres>,
    task_id: Uuid,
    project_id: Uuid,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query("DELETE FROM tasks WHERE id = $1 AND project_id = $2")
        .bind(task_id)
        .bind(project_id)
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

async fn resolve_storage_url(
    pool: &Pool<Postgres>,
    project_id: Uuid,
    storage_url: &str,
) -> Option<String> {
    // Only process storage:// URLs
    if !storage_url.starts_with("storage://") {
        return Some(storage_url.to_string());
    }
    
    let key = &storage_url[10..]; // Remove "storage://" prefix
    
    // Get project to access storage configuration
    let project = match get_project_by_id(pool, project_id).await {
        Ok(Some(project)) => project,
        _ => return None,
    };
    
    // Create storage provider
    let storage_provider = match create_storage_provider_from_project(&project).await {
        Ok(provider) => provider,
        _ => return None,
    };
    
    // Generate presigned URL with 1 hour expiry
    match storage_provider.get_presigned_url(key, 3600).await {
        Ok(url) => Some(url),
        Err(_) => None,
    }
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