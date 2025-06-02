use actix_web::{App, HttpResponse, HttpServer, Responder, web};
use sqlx::{Pool, Postgres};

mod auth;
mod projects;
mod tasks;
mod storage;
mod sync;

async fn health_check(pool: web::Data<Pool<Postgres>>) -> impl Responder {
    match sqlx::query("SELECT 1").fetch_one(pool.get_ref()).await {
        Ok(_) => HttpResponse::Ok().json(serde_json::json!({
            "status": "ok",
            "service": "fast-tag-api",
            "database": "connected"
        })),
        Err(_) => HttpResponse::ServiceUnavailable().json(serde_json::json!({
            "status": "error",
            "service": "fast-tag-api",
            "database": "disconnected"
        })),
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenvy::from_filename("api/.env").ok();

    println!("Starting API server on http://localhost:8080");

    let database_url = std::env::var("DATABASE_URL").unwrap();
    let oauth_config = match auth::OAuthConfig::from_env() {
        Ok(config) => config,
        Err(e) => {
            eprintln!("Failed to load OAuth configuration: {:?}", e);
            eprintln!("Make sure all required environment variables are set in api/.env");
            std::process::exit(1);
        }
    };

    let pool = sqlx::PgPool::connect(&database_url)
        .await
        .expect("Failed to connect to database");
    
    // Create authentication storage
    let auth_storage = auth::AuthStorage::new(pool.clone());

    // Run migrations
    println!("Running database migrations...");
    match sqlx::migrate!("./migrations").run(&pool).await {
        Ok(_) => println!("Migrations completed successfully"),
        Err(e) => {
            eprintln!("Failed to run migrations: {}", e);
            std::process::exit(1);
        }
    }
    
    // Start cleanup task for expired auth requests
    let auth_storage_cleanup = auth_storage.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(300)); // 5 minutes
        loop {
            interval.tick().await;
            if let Err(e) = auth_storage_cleanup.cleanup_expired().await {
                eprintln!("Failed to cleanup expired auth requests: {}", e);
            }
        }
    });

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(oauth_config.clone()))
            .app_data(web::Data::new(auth_storage.clone()))
            .route("/health", web::get().to(health_check))
            .route("/auth/google", web::get().to(auth::google_login))
            .route(
                "/auth/google/callback",
                web::get().to(auth::google_callback),
            )
            .route("/auth/github", web::get().to(auth::github_login))
            .route(
                "/auth/github/callback",
                web::get().to(auth::github_callback),
            )
            .route("/auth/poll/{poll_token}", web::get().to(auth::poll_auth))
            .route("/me", web::get().to(auth::get_user_info))
            .route("/projects", web::post().to(projects::create_project))
            .route("/projects", web::get().to(projects::list_projects))
            .route("/projects/{id}", web::get().to(projects::get_project))
            .route("/projects/{id}", web::put().to(projects::update_project))
            .route("/projects/{id}", web::delete().to(projects::delete_project))
            .route("/projects/{project_id}/tasks", web::post().to(tasks::create_task))
            .route("/projects/{project_id}/tasks", web::get().to(tasks::list_tasks))
            .route("/projects/{project_id}/tasks/{task_id}", web::get().to(tasks::get_task))
            .route("/projects/{project_id}/tasks/{task_id}", web::put().to(tasks::update_task))
            .route("/projects/{project_id}/tasks/{task_id}", web::delete().to(tasks::delete_task))
            .route("/projects/{project_id}/storage/upload", web::post().to(storage::handlers::upload_file))
            .route("/projects/{project_id}/storage/{key}", web::get().to(storage::handlers::download_file))
            .route("/projects/{project_id}/storage/{key}/url", web::get().to(storage::handlers::get_presigned_url))
            .route("/projects/{project_id}/storage", web::get().to(storage::handlers::list_objects))
            .route("/projects/{project_id}/sync", web::post().to(sync::sync_storage_to_tasks))
            .route("/projects/{project_id}/sync/{sync_id}", web::get().to(sync::get_sync_status))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
