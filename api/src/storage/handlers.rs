use actix_web::{web, HttpResponse, Responder, HttpRequest};
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Postgres};
use uuid::Uuid;

use crate::auth::{JwtManager, Claims};
use crate::storage::factory::create_storage_provider_from_project;

#[derive(Debug, Deserialize)]
pub struct UploadRequest {
    pub key: String,
    pub content_type: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct UploadResponse {
    pub upload_url: String,
    pub key: String,
}

#[derive(Debug, Serialize)]
pub struct DownloadResponse {
    pub download_url: String,
}

#[derive(Debug, Serialize)]
pub struct ListObjectsResponse {
    pub objects: Vec<String>,
}

pub async fn upload_file(
    req: HttpRequest,
    path: web::Path<String>,
    payload: web::Bytes,
    query: web::Query<UploadRequest>,
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

    let storage_provider = match create_storage_provider_from_project(&project).await {
        Ok(provider) => provider,
        Err(e) => return HttpResponse::InternalServerError().json(format!("Storage error: {}", e)),
    };

    match storage_provider.upload(&query.key, &payload, query.content_type.as_deref()).await {
        Ok(url) => HttpResponse::Ok().json(UploadResponse {
            upload_url: url,
            key: query.key.clone(),
        }),
        Err(e) => HttpResponse::InternalServerError().json(format!("Upload failed: {}", e)),
    }
}

pub async fn download_file(
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

    let (project_id_str, key) = path.into_inner();
    let project_id = match Uuid::parse_str(&project_id_str) {
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

    let storage_provider = match create_storage_provider_from_project(&project).await {
        Ok(provider) => provider,
        Err(e) => return HttpResponse::InternalServerError().json(format!("Storage error: {}", e)),
    };

    match storage_provider.download(&key).await {
        Ok(data) => {
            let metadata = storage_provider.get_metadata(&key).await.ok();
            let content_type = metadata
                .and_then(|m| m.content_type)
                .unwrap_or_else(|| "application/octet-stream".to_string());

            HttpResponse::Ok()
                .content_type(content_type)
                .body(data)
        }
        Err(crate::storage::StorageError::NotFound) => {
            HttpResponse::NotFound().json("File not found")
        }
        Err(e) => HttpResponse::InternalServerError().json(format!("Download failed: {}", e)),
    }
}

pub async fn get_presigned_url(
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

    let (project_id_str, key) = path.into_inner();
    let project_id = match Uuid::parse_str(&project_id_str) {
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

    let storage_provider = match create_storage_provider_from_project(&project).await {
        Ok(provider) => provider,
        Err(e) => return HttpResponse::InternalServerError().json(format!("Storage error: {}", e)),
    };

    let expires_in = req
        .headers()
        .get("x-expires-in")
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(3600); // Default 1 hour

    match storage_provider.get_presigned_url(&key, expires_in).await {
        Ok(url) => HttpResponse::Ok().json(DownloadResponse { download_url: url }),
        Err(crate::storage::StorageError::NotFound) => {
            HttpResponse::NotFound().json("File not found")
        }
        Err(e) => HttpResponse::InternalServerError().json(format!("Failed to generate URL: {}", e)),
    }
}

pub async fn list_objects(
    req: HttpRequest,
    path: web::Path<String>,
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

    let storage_provider = match create_storage_provider_from_project(&project).await {
        Ok(provider) => provider,
        Err(e) => return HttpResponse::InternalServerError().json(format!("Storage error: {}", e)),
    };

    let prefix = req
        .headers()
        .get("x-prefix")
        .and_then(|h| h.to_str().ok());

    match storage_provider.list_objects(prefix).await {
        Ok(objects) => HttpResponse::Ok().json(ListObjectsResponse { objects }),
        Err(e) => HttpResponse::InternalServerError().json(format!("Failed to list objects: {}", e)),
    }
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