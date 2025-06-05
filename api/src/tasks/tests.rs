use actix_web::{test, web, App};
use sqlx::{Pool, Postgres};
use uuid::Uuid;
use serde_json::json;
use serial_test::serial;
use crate::tasks::{create_task, list_tasks, get_task, update_task, delete_task, create_task_in_db, get_task_by_id};
use crate::test_utils;



fn create_test_jwt_token(user_id: Uuid, config: &crate::auth::OAuthConfig) -> String {
    let jwt_manager = crate::auth::JwtManager::new(&config.jwt_secret);
    let unique_email = format!("test-{}@example.com", user_id);
    jwt_manager.generate_token(&user_id.to_string(), &unique_email, "Test User").expect("Failed to create test token")
}

async fn cleanup_test_data(pool: &Pool<Postgres>, user_id: Uuid, project_id: Uuid) {
    let _ = sqlx::query!("DELETE FROM tasks WHERE project_id = $1", project_id)
        .execute(pool)
        .await;
    let _ = sqlx::query!("DELETE FROM project_members WHERE project_id = $1", project_id)
        .execute(pool)
        .await;
    let _ = sqlx::query!("DELETE FROM projects WHERE id = $1", project_id)
        .execute(pool)
        .await;
    let _ = sqlx::query!("DELETE FROM users WHERE id = $1", user_id)
        .execute(pool)
        .await;
}

#[actix_web::test]
#[serial]
async fn test_create_task_success() {
    let pool = test_utils::setup_test_db().await;
    let (user_id, project_id) = test_utils::setup_test_user_and_project(&pool).await;
    
    let config = crate::auth::OAuthConfig {
        google_client_id: "test".to_string(),
        google_client_secret: "test".to_string(),
        google_redirect_url: "test".to_string(),
        github_client_id: "test".to_string(),
        github_client_secret: "test".to_string(),
        github_redirect_url: "test".to_string(),
        jwt_secret: "test_secret".to_string(),
    };
    
    let token = create_test_jwt_token(user_id, &config);

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(config))
            .route("/projects/{project_id}/tasks", web::post().to(create_task))
    ).await;

    let req_body = json!({
        "name": "Test Task",
        "resource_url": "https://example.com/image.jpg"
    });

    let req = test::TestRequest::post()
        .uri(&format!("/projects/{}/tasks", project_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(&req_body)
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["task"]["name"], "Test Task");
    assert_eq!(body["task"]["resource_url"], "https://example.com/image.jpg");
    assert_eq!(body["task"]["status"], "pending");

    cleanup_test_data(&pool, user_id, project_id).await;
}

#[actix_web::test]
#[serial]
async fn test_create_task_invalid_name() {
    let pool = test_utils::setup_test_db().await;
    let (user_id, project_id) = test_utils::setup_test_user_and_project(&pool).await;
    
    let config = crate::auth::OAuthConfig {
        google_client_id: "test".to_string(),
        google_client_secret: "test".to_string(),
        google_redirect_url: "test".to_string(),
        github_client_id: "test".to_string(),
        github_client_secret: "test".to_string(),
        github_redirect_url: "test".to_string(),
        jwt_secret: "test_secret".to_string(),
    };
    
    let token = create_test_jwt_token(user_id, &config);

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(config))
            .route("/projects/{project_id}/tasks", web::post().to(create_task))
    ).await;

    let req_body = json!({
        "name": "",
        "resource_url": null
    });

    let req = test::TestRequest::post()
        .uri(&format!("/projects/{}/tasks", project_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(&req_body)
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 400);

    cleanup_test_data(&pool, user_id, project_id).await;
}

#[actix_web::test]
#[serial]
async fn test_list_tasks() {
    let pool = test_utils::setup_test_db().await;
    let (user_id, project_id) = test_utils::setup_test_user_and_project(&pool).await;
    
    let config = crate::auth::OAuthConfig {
        google_client_id: "test".to_string(),
        google_client_secret: "test".to_string(),
        google_redirect_url: "test".to_string(),
        github_client_id: "test".to_string(),
        github_client_secret: "test".to_string(),
        github_redirect_url: "test".to_string(),
        jwt_secret: "test_secret".to_string(),
    };
    
    let token = create_test_jwt_token(user_id, &config);

    let _task1 = create_task_in_db(&pool, project_id, "Task 1", None).await.unwrap();
    let _task2 = create_task_in_db(&pool, project_id, "Task 2", Some("https://example.com")).await.unwrap();

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(config))
            .route("/projects/{project_id}/tasks", web::get().to(list_tasks))
    ).await;

    let req = test::TestRequest::get()
        .uri(&format!("/projects/{}/tasks", project_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["tasks"].as_array().unwrap().len(), 2);

    cleanup_test_data(&pool, user_id, project_id).await;
}

#[actix_web::test]
#[serial]
async fn test_get_task() {
    let pool = test_utils::setup_test_db().await;
    let (user_id, project_id) = test_utils::setup_test_user_and_project(&pool).await;
    
    let config = crate::auth::OAuthConfig {
        google_client_id: "test".to_string(),
        google_client_secret: "test".to_string(),
        google_redirect_url: "test".to_string(),
        github_client_id: "test".to_string(),
        github_client_secret: "test".to_string(),
        github_redirect_url: "test".to_string(),
        jwt_secret: "test_secret".to_string(),
    };
    
    let token = create_test_jwt_token(user_id, &config);

    let task = create_task_in_db(&pool, project_id, "Test Task", Some("https://example.com")).await.unwrap();

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(config))
            .route("/projects/{project_id}/tasks/{task_id}", web::get().to(get_task))
    ).await;

    let req = test::TestRequest::get()
        .uri(&format!("/projects/{}/tasks/{}", project_id, task.id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["task"]["id"], task.id.to_string());
    assert_eq!(body["task"]["name"], "Test Task");

    cleanup_test_data(&pool, user_id, project_id).await;
}

#[actix_web::test]
#[serial]
async fn test_update_task() {
    let pool = test_utils::setup_test_db().await;
    let (user_id, project_id) = test_utils::setup_test_user_and_project(&pool).await;
    
    let config = crate::auth::OAuthConfig {
        google_client_id: "test".to_string(),
        google_client_secret: "test".to_string(),
        google_redirect_url: "test".to_string(),
        github_client_id: "test".to_string(),
        github_client_secret: "test".to_string(),
        github_redirect_url: "test".to_string(),
        jwt_secret: "test_secret".to_string(),
    };
    
    let token = create_test_jwt_token(user_id, &config);

    let task = create_task_in_db(&pool, project_id, "Original Task", None).await.unwrap();

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(config))
            .route("/projects/{project_id}/tasks/{task_id}", web::put().to(update_task))
    ).await;

    let req_body = json!({
        "name": "Updated Task",
        "resource_url": "https://example.com/updated.jpg",
        "status": "in_progress"
    });

    let req = test::TestRequest::put()
        .uri(&format!("/projects/{}/tasks/{}", project_id, task.id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(&req_body)
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["task"]["name"], "Updated Task");
    assert_eq!(body["task"]["status"], "in_progress");
    assert_eq!(body["task"]["resource_url"], "https://example.com/updated.jpg");

    cleanup_test_data(&pool, user_id, project_id).await;
}

#[actix_web::test]
#[serial]
async fn test_update_task_invalid_status() {
    let pool = test_utils::setup_test_db().await;
    let (user_id, project_id) = test_utils::setup_test_user_and_project(&pool).await;
    
    let config = crate::auth::OAuthConfig {
        google_client_id: "test".to_string(),
        google_client_secret: "test".to_string(),
        google_redirect_url: "test".to_string(),
        github_client_id: "test".to_string(),
        github_client_secret: "test".to_string(),
        github_redirect_url: "test".to_string(),
        jwt_secret: "test_secret".to_string(),
    };
    
    let token = create_test_jwt_token(user_id, &config);

    let task = create_task_in_db(&pool, project_id, "Test Task", None).await.unwrap();

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(config))
            .route("/projects/{project_id}/tasks/{task_id}", web::put().to(update_task))
    ).await;

    let req_body = json!({
        "name": "Updated Task",
        "resource_url": null,
        "status": "invalid_status"
    });

    let req = test::TestRequest::put()
        .uri(&format!("/projects/{}/tasks/{}", project_id, task.id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(&req_body)
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 400);

    cleanup_test_data(&pool, user_id, project_id).await;
}

#[actix_web::test]
#[serial]
async fn test_delete_task() {
    let pool = test_utils::setup_test_db().await;
    let (user_id, project_id) = test_utils::setup_test_user_and_project(&pool).await;
    
    let config = crate::auth::OAuthConfig {
        google_client_id: "test".to_string(),
        google_client_secret: "test".to_string(),
        google_redirect_url: "test".to_string(),
        github_client_id: "test".to_string(),
        github_client_secret: "test".to_string(),
        github_redirect_url: "test".to_string(),
        jwt_secret: "test_secret".to_string(),
    };
    
    let token = create_test_jwt_token(user_id, &config);

    let task = create_task_in_db(&pool, project_id, "Task to Delete", None).await.unwrap();

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(config))
            .route("/projects/{project_id}/tasks/{task_id}", web::delete().to(delete_task))
    ).await;

    let req = test::TestRequest::delete()
        .uri(&format!("/projects/{}/tasks/{}", project_id, task.id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 204);

    let deleted_task = get_task_by_id(&pool, task.id, project_id).await.unwrap();
    assert!(deleted_task.is_none());

    cleanup_test_data(&pool, user_id, project_id).await;
}

#[actix_web::test]
#[serial]
async fn test_unauthorized_access() {
    let pool = test_utils::setup_test_db().await;
    let (user_id, project_id) = test_utils::setup_test_user_and_project(&pool).await;
    
    let config = crate::auth::OAuthConfig {
        google_client_id: "test".to_string(),
        google_client_secret: "test".to_string(),
        google_redirect_url: "test".to_string(),
        github_client_id: "test".to_string(),
        github_client_secret: "test".to_string(),
        github_redirect_url: "test".to_string(),
        jwt_secret: "test_secret".to_string(),
    };

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(config))
            .route("/projects/{project_id}/tasks", web::get().to(list_tasks))
    ).await;

    let req = test::TestRequest::get()
        .uri(&format!("/projects/{}/tasks", project_id))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401);

    cleanup_test_data(&pool, user_id, project_id).await;
}

#[actix_web::test]
#[serial]
async fn test_project_not_found() {
    let pool = test_utils::setup_test_db().await;
    let (user_id, _) = test_utils::setup_test_user_and_project(&pool).await;
    
    let config = crate::auth::OAuthConfig {
        google_client_id: "test".to_string(),
        google_client_secret: "test".to_string(),
        google_redirect_url: "test".to_string(),
        github_client_id: "test".to_string(),
        github_client_secret: "test".to_string(),
        github_redirect_url: "test".to_string(),
        jwt_secret: "test_secret".to_string(),
    };
    
    let token = create_test_jwt_token(user_id, &config);
    let nonexistent_project_id = Uuid::new_v4();

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(config))
            .route("/projects/{project_id}/tasks", web::get().to(list_tasks))
    ).await;

    let req = test::TestRequest::get()
        .uri(&format!("/projects/{}/tasks", nonexistent_project_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 404);

    cleanup_test_data(&pool, user_id, Uuid::new_v4()).await;
}