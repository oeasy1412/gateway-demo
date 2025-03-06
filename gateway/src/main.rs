use actix_web::{web, App, HttpRequest, HttpServer, Responder};
use std::sync::Arc;

pub mod config;
pub mod registry;
pub mod reverse_proxy;
mod tests;

use config::get_config;
use reverse_proxy::ReverseProxy;

async fn proxy_handler(
    req:HttpRequest,
    payload:web::Payload,
    proxy:web::Data<Arc<ReverseProxy>>
) -> impl Responder {
    proxy.handle_request(req,payload).await
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();
    let port = get_config("stack.yaml").await;
    let bind_url = format!("0.0.0.0:{}", port);
    let proxy=web::Data::new(Arc::new(ReverseProxy::new()));
    HttpServer::new(move || {
        App::new()
            // .app_data(web::Data::new(config.clone()))
            .app_data(proxy.clone())
            .route("/{service_name}{path:/?.*}", web::to(proxy_handler))
    })
    .bind(bind_url)?
    .run()
    .await
}