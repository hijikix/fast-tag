use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use sqlx::{Pool, Postgres};

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
        }))
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenvy::from_filename("api/.env").ok();
    
    println!("Starting API server on http://localhost:8080");

    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://localhost/fast_tag".to_string());
    
    let pool = sqlx::PgPool::connect(&database_url)
        .await
        .expect("Failed to connect to database");

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .route("/health", web::get().to(health_check))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}