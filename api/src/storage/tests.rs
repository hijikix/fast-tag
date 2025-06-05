use actix_web::{test, web, App};
use sqlx::{Pool, Postgres};
use uuid::Uuid;
use serial_test::serial;
use bytes::Bytes;
use crate::storage::handlers::{upload_file, download_file, get_presigned_url, list_objects};
use crate::test_utils;



fn create_test_jwt_token(user_id: Uuid, config: &crate::auth::OAuthConfig) -> String {
    let jwt_manager = crate::auth::JwtManager::new(&config.jwt_secret);
    let unique_email = format!("test-{}@example.com", user_id);
    jwt_manager.generate_token(&user_id.to_string(), &unique_email, "Test User").expect("Failed to create test token")
}

async fn cleanup_test_data(pool: &Pool<Postgres>, user_id: Uuid, project_id: Uuid) {
    let _ = sqlx::query!("DELETE FROM project_members WHERE project_id = $1", project_id)
        .execute(pool)
        .await;
    let _ = sqlx::query!("DELETE FROM projects WHERE id = $1", project_id)
        .execute(pool)
        .await;
    let _ = sqlx::query!("DELETE FROM users WHERE id = $1", user_id)
        .execute(pool)
        .await;
    
    // Clean up test files
    let _ = std::fs::remove_dir_all("/tmp/fast_tag_test");
}

fn get_test_config() -> crate::auth::OAuthConfig {
    crate::auth::OAuthConfig {
        google_client_id: "test".to_string(),
        google_client_secret: "test".to_string(),
        google_redirect_url: "test".to_string(),
        github_client_id: "test".to_string(),
        github_client_secret: "test".to_string(),
        github_redirect_url: "test".to_string(),
        jwt_secret: "test_secret".to_string(),
    }
}

#[actix_web::test]
#[serial]
async fn test_upload_file_success() {
    let pool = test_utils::setup_test_db().await;
    let (user_id, project_id) = test_utils::setup_test_user_and_project_with_storage(&pool).await;
    let config = get_test_config();
    let token = create_test_jwt_token(user_id, &config);

    // Ensure test directory exists
    std::fs::create_dir_all("/tmp/fast_tag_test").unwrap();

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(config))
            .route("/projects/{project_id}/storage/upload", web::post().to(upload_file))
    ).await;

    let test_data = Bytes::from("test file content");
    let req = test::TestRequest::post()
        .uri(&format!("/projects/{}/storage/upload?key=test-file.txt&content_type=text/plain", project_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_payload(test_data)
        .to_request();

    let resp = test::call_service(&app, req).await;
    let status = resp.status();
    if status != 200 {
        let error_body = test::read_body(resp).await;
        panic!("Expected 200, got {}. Error: {}", status, String::from_utf8_lossy(&error_body));
    }

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["key"], "test-file.txt");
    assert!(body["upload_url"].as_str().is_some());

    cleanup_test_data(&pool, user_id, project_id).await;
}

#[actix_web::test]
#[serial]
async fn test_upload_file_unauthorized() {
    let pool = test_utils::setup_test_db().await;
    let (user_id, project_id) = test_utils::setup_test_user_and_project_with_storage(&pool).await;
    let config = get_test_config();

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(config))
            .route("/projects/{project_id}/storage/upload", web::post().to(upload_file))
    ).await;

    let test_data = Bytes::from("test file content");
    let req = test::TestRequest::post()
        .uri(&format!("/projects/{}/storage/upload?key=test-file.txt", project_id))
        .set_payload(test_data)
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401);

    cleanup_test_data(&pool, user_id, project_id).await;
}

#[actix_web::test]
#[serial]
async fn test_upload_file_invalid_project() {
    let pool = test_utils::setup_test_db().await;
    let (user_id, project_id) = test_utils::setup_test_user_and_project_with_storage(&pool).await;
    let config = get_test_config();
    let token = create_test_jwt_token(user_id, &config);

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(config))
            .route("/projects/{project_id}/storage/upload", web::post().to(upload_file))
    ).await;

    let test_data = Bytes::from("test file content");
    let invalid_project_id = Uuid::new_v4();
    let req = test::TestRequest::post()
        .uri(&format!("/projects/{}/storage/upload?key=test-file.txt", invalid_project_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_payload(test_data)
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 404);

    cleanup_test_data(&pool, user_id, project_id).await;
}

#[actix_web::test]
#[serial]
async fn test_download_file_success() {
    let pool = test_utils::setup_test_db().await;
    let (user_id, project_id) = test_utils::setup_test_user_and_project_with_storage(&pool).await;
    let config = get_test_config();
    let token = create_test_jwt_token(user_id, &config);

    // Setup test file
    let test_dir = "/tmp/fast_tag_test";
    std::fs::create_dir_all(test_dir).unwrap();
    let test_file_path = format!("{}/test-download.txt", test_dir);
    std::fs::write(&test_file_path, "test download content").unwrap();

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(config))
            .route("/projects/{project_id}/storage/{key}", web::get().to(download_file))
    ).await;

    let req = test::TestRequest::get()
        .uri(&format!("/projects/{}/storage/test-download.txt", project_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);

    let body = test::read_body(resp).await;
    assert_eq!(body, "test download content");

    cleanup_test_data(&pool, user_id, project_id).await;
}

#[actix_web::test]
#[serial]
async fn test_download_file_not_found() {
    let pool = test_utils::setup_test_db().await;
    let (user_id, project_id) = test_utils::setup_test_user_and_project_with_storage(&pool).await;
    let config = get_test_config();
    let token = create_test_jwt_token(user_id, &config);

    std::fs::create_dir_all("/tmp/fast_tag_test").unwrap();

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(config))
            .route("/projects/{project_id}/storage/{key}", web::get().to(download_file))
    ).await;

    let req = test::TestRequest::get()
        .uri(&format!("/projects/{}/storage/nonexistent-file.txt", project_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 404);

    cleanup_test_data(&pool, user_id, project_id).await;
}

#[actix_web::test]
#[serial]
async fn test_get_presigned_url_success() {
    let pool = test_utils::setup_test_db().await;
    let (user_id, project_id) = test_utils::setup_test_user_and_project_with_storage(&pool).await;
    let config = get_test_config();
    let token = create_test_jwt_token(user_id, &config);

    // Setup test file
    let test_dir = "/tmp/fast_tag_test";
    std::fs::create_dir_all(test_dir).unwrap();
    let test_file_path = format!("{}/test-presigned.txt", test_dir);
    std::fs::write(&test_file_path, "test presigned content").unwrap();

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(config))
            .route("/projects/{project_id}/storage/{key}/url", web::get().to(get_presigned_url))
    ).await;

    let req = test::TestRequest::get()
        .uri(&format!("/projects/{}/storage/test-presigned.txt/url", project_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .insert_header(("x-expires-in", "1800"))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body["download_url"].as_str().is_some());

    cleanup_test_data(&pool, user_id, project_id).await;
}

#[actix_web::test]
#[serial]
async fn test_get_presigned_url_file_not_found() {
    let pool = test_utils::setup_test_db().await;
    let (user_id, project_id) = test_utils::setup_test_user_and_project_with_storage(&pool).await;
    let config = get_test_config();
    let token = create_test_jwt_token(user_id, &config);

    std::fs::create_dir_all("/tmp/fast_tag_test").unwrap();

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(config))
            .route("/projects/{project_id}/storage/{key}/url", web::get().to(get_presigned_url))
    ).await;

    let req = test::TestRequest::get()
        .uri(&format!("/projects/{}/storage/nonexistent-file.txt/url", project_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 404);

    cleanup_test_data(&pool, user_id, project_id).await;
}

#[actix_web::test]
#[serial]
async fn test_list_objects_success() {
    let pool = test_utils::setup_test_db().await;
    let (user_id, project_id) = test_utils::setup_test_user_and_project_with_storage(&pool).await;
    let config = get_test_config();
    let token = create_test_jwt_token(user_id, &config);

    // Setup test files
    let test_dir = "/tmp/fast_tag_test";
    std::fs::create_dir_all(test_dir).unwrap();
    std::fs::write(format!("{}/file1.txt", test_dir), "content1").unwrap();
    std::fs::write(format!("{}/file2.txt", test_dir), "content2").unwrap();
    std::fs::create_dir_all(format!("{}/subdir", test_dir)).unwrap();
    std::fs::write(format!("{}/subdir/file3.txt", test_dir), "content3").unwrap();

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(config))
            .route("/projects/{project_id}/storage", web::get().to(list_objects))
    ).await;

    let req = test::TestRequest::get()
        .uri(&format!("/projects/{}/storage", project_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);

    let body: serde_json::Value = test::read_body_json(resp).await;
    let objects = body["objects"].as_array().unwrap();
    assert!(objects.len() >= 2);
    
    let object_names: Vec<&str> = objects.iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    assert!(object_names.contains(&"file1.txt"));
    assert!(object_names.contains(&"file2.txt"));

    cleanup_test_data(&pool, user_id, project_id).await;
}

#[actix_web::test]
#[serial]
async fn test_list_objects_with_prefix() {
    let pool = test_utils::setup_test_db().await;
    let (user_id, project_id) = test_utils::setup_test_user_and_project_with_storage(&pool).await;
    let config = get_test_config();
    let token = create_test_jwt_token(user_id, &config);

    // Setup test files with prefixes
    let test_dir = "/tmp/fast_tag_test";
    std::fs::create_dir_all(test_dir).unwrap();
    std::fs::create_dir_all(format!("{}/images", test_dir)).unwrap();
    std::fs::create_dir_all(format!("{}/docs", test_dir)).unwrap();
    std::fs::write(format!("{}/images/photo1.jpg", test_dir), "image1").unwrap();
    std::fs::write(format!("{}/images/photo2.jpg", test_dir), "image2").unwrap();
    std::fs::write(format!("{}/docs/readme.txt", test_dir), "docs").unwrap();

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(config))
            .route("/projects/{project_id}/storage", web::get().to(list_objects))
    ).await;

    let req = test::TestRequest::get()
        .uri(&format!("/projects/{}/storage", project_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .insert_header(("x-prefix", "images/"))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);

    let body: serde_json::Value = test::read_body_json(resp).await;
    let objects = body["objects"].as_array().unwrap();
    
    let object_names: Vec<&str> = objects.iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    
    // Should only contain files with images/ prefix
    for name in &object_names {
        assert!(name.starts_with("images/"));
    }

    cleanup_test_data(&pool, user_id, project_id).await;
}

#[actix_web::test]
#[serial]
async fn test_list_objects_empty_directory() {
    let pool = test_utils::setup_test_db().await;
    let (user_id, project_id) = test_utils::setup_test_user_and_project_with_storage(&pool).await;
    let config = get_test_config();
    let token = create_test_jwt_token(user_id, &config);

    // Create empty directory
    std::fs::create_dir_all("/tmp/fast_tag_test").unwrap();

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(config))
            .route("/projects/{project_id}/storage", web::get().to(list_objects))
    ).await;

    let req = test::TestRequest::get()
        .uri(&format!("/projects/{}/storage", project_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);

    let body: serde_json::Value = test::read_body_json(resp).await;
    let objects = body["objects"].as_array().unwrap();
    assert_eq!(objects.len(), 0);

    cleanup_test_data(&pool, user_id, project_id).await;
}

#[actix_web::test]
#[serial]
async fn test_storage_unauthorized_access() {
    let pool = test_utils::setup_test_db().await;
    let (user_id, project_id) = test_utils::setup_test_user_and_project_with_storage(&pool).await;
    let config = get_test_config();

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(config))
            .route("/projects/{project_id}/storage", web::get().to(list_objects))
            .route("/projects/{project_id}/storage/{key}", web::get().to(download_file))
            .route("/projects/{project_id}/storage/{key}/url", web::get().to(get_presigned_url))
            .route("/projects/{project_id}/storage/upload", web::post().to(upload_file))
    ).await;

    // Test unauthorized list
    let req = test::TestRequest::get()
        .uri(&format!("/projects/{}/storage", project_id))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401);

    // Test unauthorized download
    let req = test::TestRequest::get()
        .uri(&format!("/projects/{}/storage/test.txt", project_id))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401);

    // Test unauthorized presigned URL
    let req = test::TestRequest::get()
        .uri(&format!("/projects/{}/storage/test.txt/url", project_id))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401);

    // Test unauthorized upload
    let req = test::TestRequest::post()
        .uri(&format!("/projects/{}/storage/upload?key=test.txt", project_id))
        .set_payload(Bytes::from("test"))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401);

    cleanup_test_data(&pool, user_id, project_id).await;
}