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

#[actix_web::test]
#[serial]
async fn test_list_tasks_next_unannotated() {
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

    // Create multiple tasks
    let task1 = create_task_in_db(&pool, project_id, "Task 1", None).await.unwrap();
    let task2 = create_task_in_db(&pool, project_id, "Task 2", None).await.unwrap();
    let _task3 = create_task_in_db(&pool, project_id, "Task 3", None).await.unwrap();

    // Create annotation for task2 (middle task)
    sqlx::query!(
        "INSERT INTO annotations (id, task_id, annotated_by, annotated_at, created_at, updated_at) VALUES ($1, $2, $3, NOW(), NOW(), NOW())",
        Uuid::new_v4(),
        task2.id,
        user_id
    )
    .execute(&pool)
    .await
    .unwrap();

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(config))
            .route("/projects/{project_id}/tasks", web::get().to(list_tasks))
    ).await;

    // Test with next_unannotated=true
    let req = test::TestRequest::get()
        .uri(&format!("/projects/{}/tasks?next_unannotated=true", project_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);

    let body: serde_json::Value = test::read_body_json(resp).await;
    let tasks = body["tasks"].as_array().unwrap();
    
    // Should return only one task
    assert_eq!(tasks.len(), 1);
    
    // Should return task1 (the oldest unannotated task)
    assert_eq!(tasks[0]["id"], task1.id.to_string());
    assert_eq!(tasks[0]["name"], "Task 1");

    cleanup_test_data(&pool, user_id, project_id).await;
}

#[actix_web::test]
#[serial]
async fn test_list_tasks_next_unannotated_all_annotated() {
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

    // Create tasks
    let task1 = create_task_in_db(&pool, project_id, "Task 1", None).await.unwrap();
    let task2 = create_task_in_db(&pool, project_id, "Task 2", None).await.unwrap();

    // Create annotations for both tasks
    for task_id in &[task1.id, task2.id] {
        sqlx::query!(
            "INSERT INTO annotations (id, task_id, annotated_by, annotated_at, created_at, updated_at) VALUES ($1, $2, $3, NOW(), NOW(), NOW())",
            Uuid::new_v4(),
            *task_id,
            user_id
        )
        .execute(&pool)
        .await
        .unwrap();
    }

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(config))
            .route("/projects/{project_id}/tasks", web::get().to(list_tasks))
    ).await;

    // Test with next_unannotated=true when all tasks are annotated
    let req = test::TestRequest::get()
        .uri(&format!("/projects/{}/tasks?next_unannotated=true", project_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);

    let body: serde_json::Value = test::read_body_json(resp).await;
    let tasks = body["tasks"].as_array().unwrap();
    
    // Should return empty array when all tasks are annotated
    assert_eq!(tasks.len(), 0);

    cleanup_test_data(&pool, user_id, project_id).await;
}

#[actix_web::test]
#[serial]
async fn test_list_tasks_next_unannotated_completed_tasks() {
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

    // Create tasks
    let task1 = create_task_in_db(&pool, project_id, "Task 1", None).await.unwrap();
    let _task2 = create_task_in_db(&pool, project_id, "Task 2", None).await.unwrap();

    // Mark task1 as completed
    sqlx::query!(
        "UPDATE tasks SET status = 'completed', completed_at = NOW() WHERE id = $1",
        task1.id
    )
    .execute(&pool)
    .await
    .unwrap();

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(config))
            .route("/projects/{project_id}/tasks", web::get().to(list_tasks))
    ).await;

    // Test with next_unannotated=true
    let req = test::TestRequest::get()
        .uri(&format!("/projects/{}/tasks?next_unannotated=true", project_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);

    let body: serde_json::Value = test::read_body_json(resp).await;
    let tasks = body["tasks"].as_array().unwrap();
    
    // Should return only task2 (task1 is completed)
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0]["name"], "Task 2");

    cleanup_test_data(&pool, user_id, project_id).await;
}

#[actix_web::test]
#[serial]
async fn test_list_tasks_next_unannotated_random() {
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

    // Create multiple tasks
    let task1 = create_task_in_db(&pool, project_id, "Task 1", None).await.unwrap();
    let task2 = create_task_in_db(&pool, project_id, "Task 2", None).await.unwrap();
    let task3 = create_task_in_db(&pool, project_id, "Task 3", None).await.unwrap();

    // Create annotation for task2 (middle task)
    sqlx::query!(
        "INSERT INTO annotations (id, task_id, annotated_by, annotated_at, created_at, updated_at) VALUES ($1, $2, $3, NOW(), NOW(), NOW())",
        Uuid::new_v4(),
        task2.id,
        user_id
    )
    .execute(&pool)
    .await
    .unwrap();

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(config))
            .route("/projects/{project_id}/tasks", web::get().to(list_tasks))
    ).await;

    // Test with next_unannotated=true&random=true
    let req = test::TestRequest::get()
        .uri(&format!("/projects/{}/tasks?next_unannotated=true&random=true", project_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);

    let body: serde_json::Value = test::read_body_json(resp).await;
    let tasks = body["tasks"].as_array().unwrap();
    
    // Should return only one task
    assert_eq!(tasks.len(), 1);
    
    // Should return either task1 or task3 (both are unannotated)
    let returned_task_id = tasks[0]["id"].as_str().unwrap();
    assert!(returned_task_id == task1.id.to_string() || returned_task_id == task3.id.to_string());
    
    // Should not return task2 (it's annotated)
    assert_ne!(returned_task_id, task2.id.to_string());

    cleanup_test_data(&pool, user_id, project_id).await;
}

#[actix_web::test]
#[serial]
async fn test_list_tasks_random_without_next_unannotated() {
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

    // Create tasks
    let _task1 = create_task_in_db(&pool, project_id, "Task 1", None).await.unwrap();
    let _task2 = create_task_in_db(&pool, project_id, "Task 2", None).await.unwrap();

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(config))
            .route("/projects/{project_id}/tasks", web::get().to(list_tasks))
    ).await;

    // Test with random=true but without next_unannotated (should return all tasks)
    let req = test::TestRequest::get()
        .uri(&format!("/projects/{}/tasks?random=true", project_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);

    let body: serde_json::Value = test::read_body_json(resp).await;
    let tasks = body["tasks"].as_array().unwrap();
    
    // Should return all tasks (random flag is ignored without next_unannotated)
    assert_eq!(tasks.len(), 2);

    cleanup_test_data(&pool, user_id, project_id).await;
}