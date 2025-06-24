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

    let body: types::CocoExport = test::read_body_json(resp).await;
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

    let body: types::CocoExport = test::read_body_json(resp).await;
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
    assert_eq!(body.annotations[0].area, 30000);
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
        "licenses": [
            {
                "id": 1,
                "name": "Test License",
                "url": "https://example.com/license"
            }
        ],
        "images": [
            {
                "id": 1,
                "width": 640,
                "height": 480,
                "file_name": "test_image.jpg",
                "license": 1,
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
                "area": 30000,
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

    let body: types::ImportResult = test::read_body_json(resp).await;
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