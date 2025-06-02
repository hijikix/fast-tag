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