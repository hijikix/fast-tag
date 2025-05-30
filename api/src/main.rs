use actix_web::{web, App, HttpResponse, HttpServer, Responder};

async fn health_check() -> impl Responder {
    HttpResponse::Ok().json(serde_json::json!({
        "status": "ok",
        "service": "fast-tag-api"
    }))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    println!("Starting API server on http://localhost:8080");

    HttpServer::new(|| {
        App::new()
            .route("/health", web::get().to(health_check))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}