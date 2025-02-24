use actix_web::{get, web, App, HttpServer, Responder};
use std::env;

#[get("/")]
async fn root() -> impl Responder {
    let port = env::args()
        .nth(2)
        .unwrap_or_else(|| "unknown".to_string());
    
    web::Json(serde_json::json!({
        "status": "success",
        "port": port,
        "message": "Service started successfully"
    }))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let port = env::args()
        .nth(2)
        .and_then(|s| s.parse().ok())
        .unwrap_or(8081);

    // 显式绑定 0.0.0.0
    HttpServer::new(|| {
        App::new().service(root)
    })
    .bind(format!("0.0.0.0:{}", port))?
    .run()
    .await
}