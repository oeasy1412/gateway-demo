use actix_web::{get, post, web, App, HttpServer, Responder};
use serde::{Deserialize, Serialize};
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

/// 定义 POST /echo 的请求结构体
#[derive(Deserialize)]
struct EchoRequest {
    message: String,
}

/// 定义 POST /echo 的响应结构体
#[derive(Serialize)]
struct EchoResponse {
    status: String,
    received_message: String,
}

#[post("/echo")]
async fn echo_post_handler(request: web::Json<EchoRequest>) -> impl Responder {
    let message = &request.message;
    let response = EchoResponse {
        status: "success".to_string(),
        received_message: message.clone(),
    };

    web::Json(response)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let port = env::args()
        .nth(2)
        .and_then(|s| s.parse().ok())
        .unwrap_or(8081);

    // 显式绑定 0.0.0.0
    HttpServer::new(|| {
        App::new()
            .service(root)
            .service(echo_post_handler)
    })
    .bind(format!("0.0.0.0:{}", port))?
    .run()
    .await
}