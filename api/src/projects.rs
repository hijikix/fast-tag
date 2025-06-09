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
    pub storage_config: Option<serde_json::Value>,
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

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateProjectRequest {
    pub name: String,
    pub description: Option<String>,
    pub storage_config: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateProjectRequest {
    pub name: String,
    pub description: Option<String>,
    pub storage_config: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateStorageConfigRequest {
    pub storage_config: serde_json::Value,
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

    // Validate storage config if provided
    if let Some(storage_config) = &payload.storage_config {
        if let Err(e) = validate_storage_config(storage_config) {
            return HttpResponse::BadRequest().json(format!("Invalid storage configuration: {}", e));
        }
    }

    // Create project
    match create_project_in_db(&pool, &payload.name, payload.description.as_deref(), payload.storage_config.as_ref(), user_id).await {
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

    // Validate storage config if provided
    if let Some(storage_config) = &payload.storage_config {
        if let Err(e) = validate_storage_config(storage_config) {
            return HttpResponse::BadRequest().json(format!("Invalid storage configuration: {}", e));
        }
    }

    // Update project
    match update_project_in_db(&pool, project_id, &payload.name, payload.description.as_deref(), payload.storage_config.as_ref(), user_id).await {
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

pub async fn update_storage_config(
    req: HttpRequest,
    path: web::Path<String>,
    payload: web::Json<UpdateStorageConfigRequest>,
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

    // Validate storage config
    if let Err(e) = validate_storage_config(&payload.storage_config) {
        return HttpResponse::BadRequest().json(format!("Invalid storage configuration: {}", e));
    }

    // Update storage config
    match update_storage_config_in_db(&pool, project_id, &payload.storage_config, user_id).await {
        Ok(Some(project)) => HttpResponse::Ok().json(ProjectResponse { project }),
        Ok(None) => HttpResponse::NotFound().json("Project not found or access denied"),
        Err(_) => HttpResponse::InternalServerError().json("Failed to update storage configuration"),
    }
}

async fn create_project_in_db(
    pool: &Pool<Postgres>,
    name: &str,
    description: Option<&str>,
    storage_config: Option<&serde_json::Value>,
    owner_id: Uuid,
) -> Result<Project, sqlx::Error> {
    let project_id = Uuid::new_v4();
    let now = Utc::now();

    // Start transaction
    let mut tx = pool.begin().await?;

    // Insert project
    sqlx::query(
        "INSERT INTO projects (id, name, description, storage_config, owner_id, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7)"
    )
    .bind(project_id)
    .bind(name)
    .bind(description)
    .bind(storage_config)
    .bind(owner_id)
    .bind(now)
    .bind(now)
    .execute(&mut *tx)
    .await?;

    // Add owner as project member with 'owner' role
    let member_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO project_members (id, project_id, user_id, role, joined_at) VALUES ($1, $2, $3, $4, $5)"
    )
    .bind(member_id)
    .bind(project_id)
    .bind(owner_id)
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
        storage_config: storage_config.cloned(),
        created_at: now,
        updated_at: now,
    })
}

async fn get_user_projects(pool: &Pool<Postgres>, user_id: Uuid) -> Result<Vec<Project>, sqlx::Error> {
    sqlx::query_as::<_, Project>(
        r#"
        SELECT DISTINCT p.id, p.name, p.description, p.storage_config, p.owner_id, p.created_at, p.updated_at
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
        SELECT DISTINCT p.id, p.name, p.description, p.storage_config, p.owner_id, p.created_at, p.updated_at
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
    storage_config: Option<&serde_json::Value>,
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
        SET name = $1, description = $2, storage_config = $3, updated_at = $4
        WHERE id = $5
        RETURNING id, name, description, storage_config, owner_id, created_at, updated_at
        "#
    )
    .bind(name)
    .bind(description)
    .bind(storage_config)
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

async fn update_storage_config_in_db(
    pool: &Pool<Postgres>,
    project_id: Uuid,
    storage_config: &serde_json::Value,
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

    // Update only the storage config
    let updated_project = sqlx::query_as::<_, Project>(
        r#"
        UPDATE projects 
        SET storage_config = $1, updated_at = $2
        WHERE id = $3
        RETURNING id, name, description, storage_config, owner_id, created_at, updated_at
        "#
    )
    .bind(storage_config)
    .bind(now)
    .bind(project_id)
    .fetch_optional(pool)
    .await?;

    Ok(updated_project)
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

fn validate_storage_config(config: &serde_json::Value) -> Result<(), String> {
    use crate::storage::config::StorageConfig;
    
    let storage_config: StorageConfig = serde_json::from_value(config.clone())
        .map_err(|e| format!("Invalid JSON format: {}", e))?;
    
    storage_config.validate()
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
    async fn test_create_project_success() {
        let pool = test_utils::setup_test_db().await;
        let user = test_utils::create_test_user_with_details(&pool).await;
        let oauth_config = create_test_oauth_config();
        let token = create_auth_token(&oauth_config, &user);
        let auth_storage = AuthStorage::new(pool.clone());

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(pool.clone()))
                .app_data(web::Data::new(oauth_config))
                .app_data(web::Data::new(auth_storage))
                .route("/projects", web::post().to(create_project))
        ).await;

        let create_request = CreateProjectRequest {
            name: "Test Project".to_string(),
            description: Some("A test project".to_string()),
            storage_config: None,
        };

        let req = test::TestRequest::post()
            .uri("/projects")
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .set_json(create_request)
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 201);

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["project"]["name"], "Test Project");
        assert_eq!(body["project"]["description"], "A test project");
        assert_eq!(body["project"]["owner_id"], user.id.to_string());
    }

    #[actix_web::test]
    #[serial]
    async fn test_create_project_missing_auth() {
        let pool = test_utils::setup_test_db().await;
        let oauth_config = create_test_oauth_config();
        let auth_storage = AuthStorage::new(pool.clone());

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(pool.clone()))
                .app_data(web::Data::new(oauth_config))
                .app_data(web::Data::new(auth_storage))
                .route("/projects", web::post().to(create_project))
        ).await;

        let create_request = CreateProjectRequest {
            name: "Test Project".to_string(),
            description: None,
            storage_config: None,
        };

        let req = test::TestRequest::post()
            .uri("/projects")
            .set_json(create_request)
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 401);
    }

    #[actix_web::test]
    #[serial]
    async fn test_create_project_empty_name() {
        let pool = test_utils::setup_test_db().await;
        let user = test_utils::create_test_user_with_details(&pool).await;
        let oauth_config = create_test_oauth_config();
        let token = create_auth_token(&oauth_config, &user);
        let auth_storage = AuthStorage::new(pool.clone());

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(pool.clone()))
                .app_data(web::Data::new(oauth_config))
                .app_data(web::Data::new(auth_storage))
                .route("/projects", web::post().to(create_project))
        ).await;

        let create_request = CreateProjectRequest {
            name: "".to_string(),
            description: None,
            storage_config: None,
        };

        let req = test::TestRequest::post()
            .uri("/projects")
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .set_json(create_request)
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 400);
    }

    #[actix_web::test]
    #[serial]
    async fn test_list_projects_success() {
        let pool = test_utils::setup_test_db().await;
        let user = test_utils::create_test_user_with_details(&pool).await;
        let oauth_config = create_test_oauth_config();
        let token = create_auth_token(&oauth_config, &user);
        let auth_storage = AuthStorage::new(pool.clone());

        create_project_in_db(&pool, "Test Project 1", Some("Description 1"), None, user.id).await.unwrap();
        create_project_in_db(&pool, "Test Project 2", Some("Description 2"), None, user.id).await.unwrap();

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(pool.clone()))
                .app_data(web::Data::new(oauth_config))
                .app_data(web::Data::new(auth_storage))
                .route("/projects", web::get().to(list_projects))
        ).await;

        let req = test::TestRequest::get()
            .uri("/projects")
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["projects"].as_array().unwrap().len(), 2);
    }

    #[actix_web::test]
    #[serial]
    async fn test_get_project_success() {
        let pool = test_utils::setup_test_db().await;
        let user = test_utils::create_test_user_with_details(&pool).await;
        let oauth_config = create_test_oauth_config();
        let token = create_auth_token(&oauth_config, &user);
        let auth_storage = AuthStorage::new(pool.clone());

        let project = create_project_in_db(&pool, "Test Project", Some("Description"), None, user.id).await.unwrap();

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(pool.clone()))
                .app_data(web::Data::new(oauth_config))
                .app_data(web::Data::new(auth_storage))
                .route("/projects/{id}", web::get().to(get_project))
        ).await;

        let req = test::TestRequest::get()
            .uri(&format!("/projects/{}", project.id))
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["project"]["id"], project.id.to_string());
        assert_eq!(body["project"]["name"], "Test Project");
    }

    #[actix_web::test]
    #[serial]
    async fn test_get_project_not_found() {
        let pool = test_utils::setup_test_db().await;
        let user = test_utils::create_test_user_with_details(&pool).await;
        let oauth_config = create_test_oauth_config();
        let token = create_auth_token(&oauth_config, &user);
        let auth_storage = AuthStorage::new(pool.clone());

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(pool.clone()))
                .app_data(web::Data::new(oauth_config))
                .app_data(web::Data::new(auth_storage))
                .route("/projects/{id}", web::get().to(get_project))
        ).await;

        let random_uuid = Uuid::new_v4();
        let req = test::TestRequest::get()
            .uri(&format!("/projects/{}", random_uuid))
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 404);
    }

    #[actix_web::test]
    #[serial]
    async fn test_update_project_success() {
        let pool = test_utils::setup_test_db().await;
        let user = test_utils::create_test_user_with_details(&pool).await;
        let oauth_config = create_test_oauth_config();
        let token = create_auth_token(&oauth_config, &user);
        let auth_storage = AuthStorage::new(pool.clone());

        let project = create_project_in_db(&pool, "Test Project", Some("Original Description"), None, user.id).await.unwrap();

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(pool.clone()))
                .app_data(web::Data::new(oauth_config))
                .app_data(web::Data::new(auth_storage))
                .route("/projects/{id}", web::put().to(update_project))
        ).await;

        let update_request = UpdateProjectRequest {
            name: "Updated Project".to_string(),
            description: Some("Updated Description".to_string()),
            storage_config: None,
        };

        let req = test::TestRequest::put()
            .uri(&format!("/projects/{}", project.id))
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .set_json(update_request)
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["project"]["name"], "Updated Project");
        assert_eq!(body["project"]["description"], "Updated Description");
    }

    #[actix_web::test]
    #[serial]
    async fn test_delete_project_success() {
        let pool = test_utils::setup_test_db().await;
        let user = test_utils::create_test_user_with_details(&pool).await;
        let oauth_config = create_test_oauth_config();
        let token = create_auth_token(&oauth_config, &user);
        let auth_storage = AuthStorage::new(pool.clone());

        let project = create_project_in_db(&pool, "Test Project", Some("Description"), None, user.id).await.unwrap();

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(pool.clone()))
                .app_data(web::Data::new(oauth_config))
                .app_data(web::Data::new(auth_storage))
                .route("/projects/{id}", web::delete().to(delete_project))
        ).await;

        let req = test::TestRequest::delete()
            .uri(&format!("/projects/{}", project.id))
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 204);
    }

    #[actix_web::test]
    #[serial]
    async fn test_delete_project_not_owner() {
        let pool = test_utils::setup_test_db().await;
        let user1 = test_utils::create_test_user_with_details(&pool).await;
        
        let user2_id = Uuid::new_v4();
        let user2 = sqlx::query_as::<_, User>(
            r#"
            INSERT INTO users (id, email, name, avatar_url, provider, provider_id)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING id, email, name, avatar_url, provider, provider_id, created_at, updated_at
            "#
        )
        .bind(user2_id)
        .bind("test2@example.com")
        .bind("Test User 2")
        .bind(None::<String>)
        .bind("google")
        .bind("google-id-456")
        .fetch_one(&pool)
        .await
        .expect("Failed to create test user 2");

        let oauth_config = create_test_oauth_config();
        let token = create_auth_token(&oauth_config, &user2);
        let auth_storage = AuthStorage::new(pool.clone());

        let project = create_project_in_db(&pool, "Test Project", Some("Description"), None, user1.id).await.unwrap();

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(pool.clone()))
                .app_data(web::Data::new(oauth_config))
                .app_data(web::Data::new(auth_storage))
                .route("/projects/{id}", web::delete().to(delete_project))
        ).await;

        let req = test::TestRequest::delete()
            .uri(&format!("/projects/{}", project.id))
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 404);
    }

    #[actix_web::test]
    #[serial]
    async fn test_update_storage_config_success() {
        let pool = test_utils::setup_test_db().await;
        let user = test_utils::create_test_user_with_details(&pool).await;
        let oauth_config = create_test_oauth_config();
        let token = create_auth_token(&oauth_config, &user);
        let auth_storage = AuthStorage::new(pool.clone());

        let project = create_project_in_db(&pool, "Test Project", Some("Description"), None, user.id).await.unwrap();

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(pool.clone()))
                .app_data(web::Data::new(oauth_config))
                .app_data(web::Data::new(auth_storage))
                .route("/projects/{id}/storage-config", web::put().to(update_storage_config))
        ).await;

        let storage_config = serde_json::json!({
            "type": "s3",
            "region": "us-east-1",
            "bucket": "test-bucket",
            "access_key": "test-access-key",
            "secret_key": "test-secret-key"
        });

        let update_request = UpdateStorageConfigRequest {
            storage_config: storage_config.clone(),
        };

        let req = test::TestRequest::put()
            .uri(&format!("/projects/{}/storage-config", project.id))
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .set_json(update_request)
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["project"]["storage_config"], storage_config);
    }

    #[actix_web::test]
    #[serial]
    async fn test_update_storage_config_invalid_config() {
        let pool = test_utils::setup_test_db().await;
        let user = test_utils::create_test_user_with_details(&pool).await;
        let oauth_config = create_test_oauth_config();
        let token = create_auth_token(&oauth_config, &user);
        let auth_storage = AuthStorage::new(pool.clone());

        let project = create_project_in_db(&pool, "Test Project", Some("Description"), None, user.id).await.unwrap();

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(pool.clone()))
                .app_data(web::Data::new(oauth_config))
                .app_data(web::Data::new(auth_storage))
                .route("/projects/{id}/storage-config", web::put().to(update_storage_config))
        ).await;

        let invalid_storage_config = serde_json::json!({
            "type": "invalid_provider"
        });

        let update_request = UpdateStorageConfigRequest {
            storage_config: invalid_storage_config,
        };

        let req = test::TestRequest::put()
            .uri(&format!("/projects/{}/storage-config", project.id))
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .set_json(update_request)
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 400);
    }

    #[actix_web::test]
    #[serial]
    async fn test_update_storage_config_not_owner() {
        let pool = test_utils::setup_test_db().await;
        let user1 = test_utils::create_test_user_with_details(&pool).await;
        
        let user2_id = Uuid::new_v4();
        let user2 = sqlx::query_as::<_, User>(
            r#"
            INSERT INTO users (id, email, name, avatar_url, provider, provider_id)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING id, email, name, avatar_url, provider, provider_id, created_at, updated_at
            "#
        )
        .bind(user2_id)
        .bind("test2@example.com")
        .bind("Test User 2")
        .bind(None::<String>)
        .bind("google")
        .bind("google-id-456")
        .fetch_one(&pool)
        .await
        .expect("Failed to create test user 2");

        let oauth_config = create_test_oauth_config();
        let token = create_auth_token(&oauth_config, &user2);
        let auth_storage = AuthStorage::new(pool.clone());

        let project = create_project_in_db(&pool, "Test Project", Some("Description"), None, user1.id).await.unwrap();

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(pool.clone()))
                .app_data(web::Data::new(oauth_config))
                .app_data(web::Data::new(auth_storage))
                .route("/projects/{id}/storage-config", web::put().to(update_storage_config))
        ).await;

        let storage_config = serde_json::json!({
            "type": "s3",
            "region": "us-east-1",
            "bucket": "test-bucket",
            "access_key": "test-access-key",
            "secret_key": "test-secret-key"
        });

        let update_request = UpdateStorageConfigRequest {
            storage_config,
        };

        let req = test::TestRequest::put()
            .uri(&format!("/projects/{}/storage-config", project.id))
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .set_json(update_request)
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 404);
    }
}