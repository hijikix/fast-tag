use actix_web::{web, HttpResponse, Responder, HttpRequest};
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Postgres};
use uuid::Uuid;
use chrono::{DateTime, Utc};

use crate::auth::{JwtManager, Claims};

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct ImageAnnotationCategory {
    pub id: Uuid,
    pub project_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub supercategory: Option<String>,
    pub color: Option<String>,
    pub coco_id: Option<i32>,
    pub image_metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}


#[derive(Debug, Serialize, Deserialize)]
pub struct CreateImageAnnotationCategoryRequest {
    pub name: String,
    pub description: Option<String>,
    pub supercategory: Option<String>,
    pub color: Option<String>,
    pub coco_id: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateImageAnnotationCategoryRequest {
    pub name: String,
    pub description: Option<String>,
    pub supercategory: Option<String>,
    pub color: Option<String>,
    pub coco_id: Option<i32>,
}

#[derive(Debug, Serialize)]
pub struct ImageAnnotationCategoryResponse {
    pub category: ImageAnnotationCategory,
}


#[derive(Debug, Serialize)]
pub struct ImageAnnotationCategoriesListResponse {
    pub categories: Vec<ImageAnnotationCategory>,
}

pub async fn create_image_annotation_category(
    req: HttpRequest,
    path: web::Path<String>,
    payload: web::Json<CreateImageAnnotationCategoryRequest>,
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
        return HttpResponse::BadRequest().json("Category name cannot be empty");
    }

    if payload.name.len() > 255 {
        return HttpResponse::BadRequest().json("Category name too long (max 255 characters)");
    }

    // Validate color format if provided
    if let Some(ref color) = payload.color {
        if !color.starts_with('#') || color.len() != 7 {
            return HttpResponse::BadRequest().json("Color must be in HEX format (#RRGGBB)");
        }
    }

    // Check if user has access to this project
    if !user_has_project_access(&pool, project_id, user_id).await {
        return HttpResponse::NotFound().json("Project not found or access denied");
    }

    // Create annotation category
    match create_image_annotation_category_in_db(
        &pool,
        project_id,
        &payload.name,
        payload.description.as_deref(),
        payload.supercategory.as_deref(),
        payload.color.as_deref(),
        payload.coco_id,
    ).await {
        Ok(category) => {
            HttpResponse::Created().json(ImageAnnotationCategoryResponse {
                category,
            })
        },
        Err(sqlx::Error::Database(db_err)) if db_err.is_unique_violation() => {
            HttpResponse::Conflict().json("Category name already exists in this project")
        }
        Err(_) => HttpResponse::InternalServerError().json("Failed to create annotation category"),
    }
}

pub async fn list_image_annotation_categories(
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

    // Get project's annotation categories
    match get_project_image_annotation_categories(&pool, project_id).await {
        Ok(categories) => HttpResponse::Ok().json(ImageAnnotationCategoriesListResponse { categories }),
        Err(_) => HttpResponse::InternalServerError().json("Failed to fetch annotation categories"),
    }
}

pub async fn get_image_annotation_category(
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

    let (project_id_str, category_id_str) = path.into_inner();
    let project_id = match Uuid::parse_str(&project_id_str) {
        Ok(id) => id,
        Err(_) => return HttpResponse::BadRequest().json("Invalid project ID"),
    };

    let category_id = match Uuid::parse_str(&category_id_str) {
        Ok(id) => id,
        Err(_) => return HttpResponse::BadRequest().json("Invalid category ID"),
    };

    // Check if user has access to this project
    if !user_has_project_access(&pool, project_id, user_id).await {
        return HttpResponse::NotFound().json("Project not found or access denied");
    }

    // Get annotation category
    match get_image_annotation_category_by_id(&pool, category_id, project_id).await {
        Ok(Some(category)) => {
            HttpResponse::Ok().json(ImageAnnotationCategoryResponse {
                category,
            })
        },
        Ok(None) => HttpResponse::NotFound().json("Annotation category not found"),
        Err(_) => HttpResponse::InternalServerError().json("Failed to fetch annotation category"),
    }
}

pub async fn update_image_annotation_category(
    req: HttpRequest,
    path: web::Path<(String, String)>,
    payload: web::Json<UpdateImageAnnotationCategoryRequest>,
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

    let (project_id_str, category_id_str) = path.into_inner();
    let project_id = match Uuid::parse_str(&project_id_str) {
        Ok(id) => id,
        Err(_) => return HttpResponse::BadRequest().json("Invalid project ID"),
    };

    let category_id = match Uuid::parse_str(&category_id_str) {
        Ok(id) => id,
        Err(_) => return HttpResponse::BadRequest().json("Invalid category ID"),
    };

    // Validate input
    if payload.name.trim().is_empty() {
        return HttpResponse::BadRequest().json("Category name cannot be empty");
    }

    if payload.name.len() > 255 {
        return HttpResponse::BadRequest().json("Category name too long (max 255 characters)");
    }

    // Validate color format if provided
    if let Some(ref color) = payload.color {
        if !color.starts_with('#') || color.len() != 7 {
            return HttpResponse::BadRequest().json("Color must be in HEX format (#RRGGBB)");
        }
    }

    // Check if user has access to this project
    if !user_has_project_access(&pool, project_id, user_id).await {
        return HttpResponse::NotFound().json("Project not found or access denied");
    }

    // Update annotation category
    match update_image_annotation_category_in_db(
        &pool,
        category_id,
        project_id,
        &payload.name,
        payload.description.as_deref(),
        payload.supercategory.as_deref(),
        payload.color.as_deref(),
        payload.coco_id,
    ).await {
        Ok(Some(category)) => {
            HttpResponse::Ok().json(ImageAnnotationCategoryResponse {
                category,
            })
        },
        Ok(None) => HttpResponse::NotFound().json("Annotation category not found"),
        Err(sqlx::Error::Database(db_err)) if db_err.is_unique_violation() => {
            HttpResponse::Conflict().json("Category name already exists in this project")
        }
        Err(_) => HttpResponse::InternalServerError().json("Failed to update annotation category"),
    }
}

pub async fn delete_image_annotation_category(
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

    let (project_id_str, category_id_str) = path.into_inner();
    let project_id = match Uuid::parse_str(&project_id_str) {
        Ok(id) => id,
        Err(_) => return HttpResponse::BadRequest().json("Invalid project ID"),
    };

    let category_id = match Uuid::parse_str(&category_id_str) {
        Ok(id) => id,
        Err(_) => return HttpResponse::BadRequest().json("Invalid category ID"),
    };

    // Check if user has access to this project
    if !user_has_project_access(&pool, project_id, user_id).await {
        return HttpResponse::NotFound().json("Project not found or access denied");
    }

    // Delete annotation category
    match delete_image_annotation_category_from_db(&pool, category_id, project_id).await {
        Ok(true) => HttpResponse::NoContent().finish(),
        Ok(false) => HttpResponse::NotFound().json("Annotation category not found"),
        Err(_) => HttpResponse::InternalServerError().json("Failed to delete annotation category"),
    }
}

pub async fn create_image_annotation_category_in_db(
    pool: &Pool<Postgres>,
    project_id: Uuid,
    name: &str,
    description: Option<&str>,
    supercategory: Option<&str>,
    color: Option<&str>,
    coco_id: Option<i32>,
) -> Result<ImageAnnotationCategory, sqlx::Error> {
    let category_id = Uuid::new_v4();
    let now = Utc::now();

    // Create annotation category in image_annotation_categories table
    let category = sqlx::query_as::<_, ImageAnnotationCategory>(
        r#"
        INSERT INTO image_annotation_categories (id, project_id, name, description, supercategory, color, coco_id, image_metadata, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
        RETURNING id, project_id, name, description, supercategory, color, coco_id, image_metadata, created_at, updated_at
        "#
    )
    .bind(category_id)
    .bind(project_id)
    .bind(name)
    .bind(description)
    .bind(supercategory)
    .bind(color)
    .bind(coco_id)
    .bind(serde_json::json!({}))
    .bind(now)
    .bind(now)
    .fetch_one(pool)
    .await?;

    Ok(category)
}

async fn get_project_image_annotation_categories(
    pool: &Pool<Postgres>,
    project_id: Uuid,
) -> Result<Vec<ImageAnnotationCategory>, sqlx::Error> {
    sqlx::query_as::<_, ImageAnnotationCategory>(
        r#"
        SELECT id, project_id, name, description, supercategory, color, coco_id, image_metadata, created_at, updated_at
        FROM image_annotation_categories
        WHERE project_id = $1
        ORDER BY name ASC
        "#
    )
    .bind(project_id)
    .fetch_all(pool)
    .await
}

async fn get_image_annotation_category_by_id(
    pool: &Pool<Postgres>,
    category_id: Uuid,
    project_id: Uuid,
) -> Result<Option<ImageAnnotationCategory>, sqlx::Error> {
    sqlx::query_as::<_, ImageAnnotationCategory>(
        r#"
        SELECT id, project_id, name, description, supercategory, color, coco_id, image_metadata, created_at, updated_at
        FROM image_annotation_categories
        WHERE id = $1 AND project_id = $2
        "#
    )
    .bind(category_id)
    .bind(project_id)
    .fetch_optional(pool)
    .await
}

#[allow(clippy::too_many_arguments)]
async fn update_image_annotation_category_in_db(
    pool: &Pool<Postgres>,
    category_id: Uuid,
    project_id: Uuid,
    name: &str,
    description: Option<&str>,
    supercategory: Option<&str>,
    color: Option<&str>,
    coco_id: Option<i32>,
) -> Result<Option<ImageAnnotationCategory>, sqlx::Error> {
    let now = Utc::now();

    // Update annotation category in image_annotation_categories table
    sqlx::query_as::<_, ImageAnnotationCategory>(
        r#"
        UPDATE image_annotation_categories
        SET name = $1, description = $2, supercategory = $3, color = $4, coco_id = $5, updated_at = $6
        WHERE id = $7 AND project_id = $8
        RETURNING id, project_id, name, description, supercategory, color, coco_id, image_metadata, created_at, updated_at
        "#
    )
    .bind(name)
    .bind(description)
    .bind(supercategory)
    .bind(color)
    .bind(coco_id)
    .bind(now)
    .bind(category_id)
    .bind(project_id)
    .fetch_optional(pool)
    .await
}

async fn delete_image_annotation_category_from_db(
    pool: &Pool<Postgres>,
    category_id: Uuid,
    project_id: Uuid,
) -> Result<bool, sqlx::Error> {
    // Delete from image_annotation_categories
    let result = sqlx::query(
        "DELETE FROM image_annotation_categories WHERE id = $1 AND project_id = $2"
    )
    .bind(category_id)
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
    async fn test_create_annotation_category_success() {
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
                .route("/projects/{project_id}/image-annotation-categories", web::post().to(create_image_annotation_category))
        ).await;

        let create_request = CreateImageAnnotationCategoryRequest {
            name: "person".to_string(),
            supercategory: Some("human".to_string()),
            color: Some("#FF0000".to_string()),
            description: Some("Human person category".to_string()),
            coco_id: Some(1),
        };

        let req = test::TestRequest::post()
            .uri(&format!("/projects/{}/image-annotation-categories", project.id))
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .set_json(create_request)
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 201);

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["category"]["name"], "person");
        assert_eq!(body["category"]["supercategory"], "human");
        assert_eq!(body["category"]["color"], "#FF0000");
        assert_eq!(body["category"]["coco_id"], 1);
    }

    #[actix_web::test]
    #[serial]
    async fn test_create_annotation_category_empty_name() {
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
                .route("/projects/{project_id}/image-annotation-categories", web::post().to(create_image_annotation_category))
        ).await;

        let create_request = CreateImageAnnotationCategoryRequest {
            name: "".to_string(),
            supercategory: None,
            color: None,
            description: None,
            coco_id: None,
        };

        let req = test::TestRequest::post()
            .uri(&format!("/projects/{}/image-annotation-categories", project.id))
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .set_json(create_request)
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 400);
    }

    #[actix_web::test]
    #[serial]
    async fn test_create_annotation_category_invalid_color() {
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
                .route("/projects/{project_id}/image-annotation-categories", web::post().to(create_image_annotation_category))
        ).await;

        let create_request = CreateImageAnnotationCategoryRequest {
            name: "person".to_string(),
            supercategory: None,
            color: Some("invalid_color".to_string()),
            description: None,
            coco_id: None,
        };

        let req = test::TestRequest::post()
            .uri(&format!("/projects/{}/image-annotation-categories", project.id))
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .set_json(create_request)
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 400);
    }

    #[actix_web::test]
    #[serial]
    async fn test_list_annotation_categories_success() {
        let pool = test_utils::setup_test_db().await;
        let user = test_utils::create_test_user_with_details(&pool).await;
        let oauth_config = create_test_oauth_config();
        let token = create_auth_token(&oauth_config, &user);
        let auth_storage = AuthStorage::new(pool.clone());

        let project = crate::projects::create_project_in_db(&pool, "Test Project", Some("Description"), None, user.id).await.unwrap();

        // Create test categories
        create_image_annotation_category_in_db(&pool, project.id, "person", None, Some("human"), Some("#FF0000"), Some(1)).await.unwrap();
        create_image_annotation_category_in_db(&pool, project.id, "car", None, Some("vehicle"), Some("#00FF00"), Some(2)).await.unwrap();

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(pool.clone()))
                .app_data(web::Data::new(oauth_config))
                .app_data(web::Data::new(auth_storage))
                .route("/projects/{project_id}/image-annotation-categories", web::get().to(list_image_annotation_categories))
        ).await;

        let req = test::TestRequest::get()
            .uri(&format!("/projects/{}/image-annotation-categories", project.id))
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["categories"].as_array().unwrap().len(), 2);
    }

    #[actix_web::test]
    #[serial]
    async fn test_get_annotation_category_success() {
        let pool = test_utils::setup_test_db().await;
        let user = test_utils::create_test_user_with_details(&pool).await;
        let oauth_config = create_test_oauth_config();
        let token = create_auth_token(&oauth_config, &user);
        let auth_storage = AuthStorage::new(pool.clone());

        let project = crate::projects::create_project_in_db(&pool, "Test Project", Some("Description"), None, user.id).await.unwrap();
        let category = create_image_annotation_category_in_db(&pool, project.id, "person", None, Some("human"), Some("#FF0000"), Some(1)).await.unwrap();

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(pool.clone()))
                .app_data(web::Data::new(oauth_config))
                .app_data(web::Data::new(auth_storage))
                .route("/projects/{project_id}/image-annotation-categories/{category_id}", web::get().to(get_image_annotation_category))
        ).await;

        let req = test::TestRequest::get()
            .uri(&format!("/projects/{}/image-annotation-categories/{}", project.id, category.id))
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["category"]["name"], "person");
        assert_eq!(body["category"]["supercategory"], "human");
    }

    #[actix_web::test]
    #[serial]
    async fn test_update_annotation_category_success() {
        let pool = test_utils::setup_test_db().await;
        let user = test_utils::create_test_user_with_details(&pool).await;
        let oauth_config = create_test_oauth_config();
        let token = create_auth_token(&oauth_config, &user);
        let auth_storage = AuthStorage::new(pool.clone());

        let project = crate::projects::create_project_in_db(&pool, "Test Project", Some("Description"), None, user.id).await.unwrap();
        let category = create_image_annotation_category_in_db(&pool, project.id, "person", None, Some("human"), Some("#FF0000"), Some(1)).await.unwrap();

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(pool.clone()))
                .app_data(web::Data::new(oauth_config))
                .app_data(web::Data::new(auth_storage))
                .route("/projects/{project_id}/image-annotation-categories/{category_id}", web::put().to(update_image_annotation_category))
        ).await;

        let update_request = UpdateImageAnnotationCategoryRequest {
            name: "human".to_string(),
            supercategory: Some("living_being".to_string()),
            color: Some("#0000FF".to_string()),
            description: Some("Updated description".to_string()),
            coco_id: Some(10),
        };

        let req = test::TestRequest::put()
            .uri(&format!("/projects/{}/image-annotation-categories/{}", project.id, category.id))
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .set_json(update_request)
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["category"]["name"], "human");
        assert_eq!(body["category"]["supercategory"], "living_being");
        assert_eq!(body["category"]["color"], "#0000FF");
    }

    #[actix_web::test]
    #[serial]
    async fn test_delete_annotation_category_success() {
        let pool = test_utils::setup_test_db().await;
        let user = test_utils::create_test_user_with_details(&pool).await;
        let oauth_config = create_test_oauth_config();
        let token = create_auth_token(&oauth_config, &user);
        let auth_storage = AuthStorage::new(pool.clone());

        let project = crate::projects::create_project_in_db(&pool, "Test Project", Some("Description"), None, user.id).await.unwrap();
        let category = create_image_annotation_category_in_db(&pool, project.id, "person", None, Some("human"), Some("#FF0000"), Some(1)).await.unwrap();

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(pool.clone()))
                .app_data(web::Data::new(oauth_config))
                .app_data(web::Data::new(auth_storage))
                .route("/projects/{project_id}/image-annotation-categories/{category_id}", web::delete().to(delete_image_annotation_category))
        ).await;

        let req = test::TestRequest::delete()
            .uri(&format!("/projects/{}/image-annotation-categories/{}", project.id, category.id))
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 204);
    }

    #[actix_web::test]
    #[serial]
    async fn test_create_annotation_category_unauthorized() {
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
                .route("/projects/{project_id}/image-annotation-categories", web::post().to(create_image_annotation_category))
        ).await;

        let create_request = CreateImageAnnotationCategoryRequest {
            name: "person".to_string(),
            supercategory: None,
            color: None,
            description: None,
            coco_id: None,
        };

        let req = test::TestRequest::post()
            .uri(&format!("/projects/{}/image-annotation-categories", project.id))
            .set_json(create_request)
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 401);
    }
}