use actix_web::{web, HttpResponse, Responder, HttpRequest};
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Postgres};
use uuid::Uuid;
use chrono::{DateTime, Utc};

use crate::auth::{JwtManager, Claims};

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Project {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub owner_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct ProjectMember {
    pub id: Uuid,
    pub project_id: Uuid,
    pub user_id: Uuid,
    pub role: String,
    pub joined_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateProjectRequest {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateProjectRequest {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ProjectResponse {
    pub project: Project,
}

#[derive(Debug, Serialize)]
pub struct ProjectsListResponse {
    pub projects: Vec<Project>,
}

pub async fn create_project(
    req: HttpRequest,
    payload: web::Json<CreateProjectRequest>,
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

    // Validate input
    if payload.name.trim().is_empty() {
        return HttpResponse::BadRequest().json("Project name cannot be empty");
    }

    if payload.name.len() > 255 {
        return HttpResponse::BadRequest().json("Project name too long (max 255 characters)");
    }

    // Create project
    match create_project_in_db(&pool, &payload.name, payload.description.as_deref(), user_id).await {
        Ok(project) => HttpResponse::Created().json(ProjectResponse { project }),
        Err(sqlx::Error::Database(db_err)) if db_err.is_unique_violation() => {
            HttpResponse::Conflict().json("Project name already exists for this user")
        }
        Err(_) => HttpResponse::InternalServerError().json("Failed to create project"),
    }
}

pub async fn list_projects(
    req: HttpRequest,
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

    // Get user's projects (owned + member of)
    match get_user_projects(&pool, user_id).await {
        Ok(projects) => HttpResponse::Ok().json(ProjectsListResponse { projects }),
        Err(_) => HttpResponse::InternalServerError().json("Failed to fetch projects"),
    }
}

pub async fn get_project(
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
    match get_project_by_id(&pool, project_id, user_id).await {
        Ok(Some(project)) => HttpResponse::Ok().json(ProjectResponse { project }),
        Ok(None) => HttpResponse::NotFound().json("Project not found or access denied"),
        Err(_) => HttpResponse::InternalServerError().json("Failed to fetch project"),
    }
}

pub async fn update_project(
    req: HttpRequest,
    path: web::Path<String>,
    payload: web::Json<UpdateProjectRequest>,
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
        return HttpResponse::BadRequest().json("Project name cannot be empty");
    }

    if payload.name.len() > 255 {
        return HttpResponse::BadRequest().json("Project name too long (max 255 characters)");
    }

    // Update project
    match update_project_in_db(&pool, project_id, &payload.name, payload.description.as_deref(), user_id).await {
        Ok(Some(project)) => HttpResponse::Ok().json(ProjectResponse { project }),
        Ok(None) => HttpResponse::NotFound().json("Project not found or access denied"),
        Err(sqlx::Error::Database(db_err)) if db_err.is_unique_violation() => {
            HttpResponse::Conflict().json("Project name already exists for this user")
        }
        Err(_) => HttpResponse::InternalServerError().json("Failed to update project"),
    }
}

pub async fn delete_project(
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

    // Delete project (only owner can delete)
    match delete_project_from_db(&pool, project_id, user_id).await {
        Ok(true) => HttpResponse::NoContent().finish(),
        Ok(false) => HttpResponse::NotFound().json("Project not found or access denied"),
        Err(_) => HttpResponse::InternalServerError().json("Failed to delete project"),
    }
}

async fn create_project_in_db(
    pool: &Pool<Postgres>,
    name: &str,
    description: Option<&str>,
    owner_id: Uuid,
) -> Result<Project, sqlx::Error> {
    let project_id = Uuid::new_v4();
    let now = Utc::now();

    // Start transaction
    let mut tx = pool.begin().await?;

    // Insert project
    sqlx::query(
        "INSERT INTO projects (id, name, description, owner_id, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6)"
    )
    .bind(&project_id)
    .bind(name)
    .bind(description)
    .bind(&owner_id)
    .bind(now)
    .bind(now)
    .execute(&mut *tx)
    .await?;

    // Add owner as project member with 'owner' role
    let member_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO project_members (id, project_id, user_id, role, joined_at) VALUES ($1, $2, $3, $4, $5)"
    )
    .bind(&member_id)
    .bind(&project_id)
    .bind(&owner_id)
    .bind("owner")
    .bind(now)
    .execute(&mut *tx)
    .await?;

    // Commit transaction
    tx.commit().await?;

    Ok(Project {
        id: project_id,
        name: name.to_string(),
        description: description.map(String::from),
        owner_id,
        created_at: now,
        updated_at: now,
    })
}

async fn get_user_projects(pool: &Pool<Postgres>, user_id: Uuid) -> Result<Vec<Project>, sqlx::Error> {
    sqlx::query_as::<_, Project>(
        r#"
        SELECT DISTINCT p.id, p.name, p.description, p.owner_id, p.created_at, p.updated_at
        FROM projects p
        INNER JOIN project_members pm ON p.id = pm.project_id
        WHERE pm.user_id = $1
        ORDER BY p.created_at DESC
        "#
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
}

async fn get_project_by_id(
    pool: &Pool<Postgres>,
    project_id: Uuid,
    user_id: Uuid,
) -> Result<Option<Project>, sqlx::Error> {
    sqlx::query_as::<_, Project>(
        r#"
        SELECT DISTINCT p.id, p.name, p.description, p.owner_id, p.created_at, p.updated_at
        FROM projects p
        INNER JOIN project_members pm ON p.id = pm.project_id
        WHERE p.id = $1 AND pm.user_id = $2
        "#
    )
    .bind(project_id)
    .bind(user_id)
    .fetch_optional(pool)
    .await
}

async fn update_project_in_db(
    pool: &Pool<Postgres>,
    project_id: Uuid,
    name: &str,
    description: Option<&str>,
    user_id: Uuid,
) -> Result<Option<Project>, sqlx::Error> {
    let now = Utc::now();

    // Check if user has owner role for this project
    let has_permission = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS(
            SELECT 1 FROM project_members pm
            INNER JOIN projects p ON pm.project_id = p.id
            WHERE p.id = $1 AND pm.user_id = $2 AND (pm.role = 'owner' OR p.owner_id = $2)
        )
        "#
    )
    .bind(project_id)
    .bind(user_id)
    .fetch_one(pool)
    .await?;

    if !has_permission {
        return Ok(None);
    }

    // Update the project
    let updated_project = sqlx::query_as::<_, Project>(
        r#"
        UPDATE projects 
        SET name = $1, description = $2, updated_at = $3
        WHERE id = $4
        RETURNING id, name, description, owner_id, created_at, updated_at
        "#
    )
    .bind(name)
    .bind(description)
    .bind(now)
    .bind(project_id)
    .fetch_optional(pool)
    .await?;

    Ok(updated_project)
}

async fn delete_project_from_db(
    pool: &Pool<Postgres>,
    project_id: Uuid,
    user_id: Uuid,
) -> Result<bool, sqlx::Error> {
    // Check if user is the owner of this project
    let is_owner = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM projects WHERE id = $1 AND owner_id = $2)"
    )
    .bind(project_id)
    .bind(user_id)
    .fetch_one(pool)
    .await?;

    if !is_owner {
        return Ok(false);
    }

    // Start transaction
    let mut tx = pool.begin().await?;

    // Delete project members first (foreign key constraint)
    sqlx::query("DELETE FROM project_members WHERE project_id = $1")
        .bind(project_id)
        .execute(&mut *tx)
        .await?;

    // Delete the project
    let result = sqlx::query("DELETE FROM projects WHERE id = $1")
        .bind(project_id)
        .execute(&mut *tx)
        .await?;

    // Commit transaction
    tx.commit().await?;

    Ok(result.rows_affected() > 0)
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