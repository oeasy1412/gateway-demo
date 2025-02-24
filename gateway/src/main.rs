use std::collections::HashMap;
use std::sync::{Arc, Mutex, atomic::{AtomicU16, Ordering}};
use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use reqwest::Client;
use url::Url;
use lazy_static::lazy_static;
use std::process::{Command, Stdio};
use std::net::{TcpListener};
use std::time::Duration;
use std::thread::sleep;

lazy_static! {
    static ref SERVICE_REGISTRY: Arc<Mutex<HashMap<String, u16>>> = 
        Arc::new(Mutex::new(HashMap::new()));
}

 static NEXT_PORT: AtomicU16 = AtomicU16::new(8050);

// 统一使用 0.0.0.0 检测端口
fn find_available_port(start_port: u16) -> Option<u16> {
    (start_port..8100).find(|port| {
        TcpListener::bind(("0.0.0.0", *port)).is_ok()
    })
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();
    
    HttpServer::new(|| {
        App::new()
            .route("/{service_name}/{path:.*}", web::get().to(handle_request))
    })
    .bind("0.0.0.0:8090")?
    .run()
    .await
}

async fn handle_request(path: web::Path<(String, String)>) -> impl Responder {
    let (service_name, rest_path) = path.into_inner();
    
    let port = match get_or_start_service(&service_name).await {
        Ok(p) => p,
        Err(e) => return HttpResponse::InternalServerError().body(e),
    };

    let rest_path = if rest_path.is_empty() { 
        "".to_string() 
    } else { 
        format!("/{}", rest_path) 
    };
    
    let target_url = format!("http://0.0.0.0:{}{}", port, rest_path);
    
    proxy_request(&target_url).await
}

async fn get_or_start_service(service_name: &str) -> Result<u16, String> {
    let mut registry = SERVICE_REGISTRY.lock().unwrap();
    
    if let Some(port) = registry.get(service_name) {
        return Ok(*port);
    }

    let start_port = NEXT_PORT.fetch_add(1, Ordering::SeqCst);
    let port = find_available_port(start_port)
        .ok_or("No available ports in range 8050-8100")?;

    println!("🔍 Verifying port {} availability...", port);
    match TcpListener::bind(("0.0.0.0", port)) {
        Ok(_) => println!("✅ Port {} is available", port),
        Err(e) => return Err(format!("Port {} check failed: {:?}", port, e)),
    }

    println!("🚀 Starting {} service on port {}", service_name, port);
    let mut child = Command::new("cargo")
        .args(&["run", "--bin", "echo", "--", "--port", &port.to_string()])
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to start service: {}", e))?;

    // 等待服务绑定端口
    sleep(Duration::from_secs(1));
    
    // 二次验证端口占用
    match TcpListener::bind(("0.0.0.0", port)) {
        Ok(_) => {
            child.kill().ok();
            return Err(format!("Port {} not occupied after startup", port));
        }
        Err(_) => println!("✅ Port {} occupied successfully", port),
    }

    registry.insert(service_name.to_string(), port);
    Ok(port)
}

async fn proxy_request(target_url: &str) -> HttpResponse {
    let client = Client::new();
    let url = Url::parse(target_url).unwrap();

    match client.get(url).send().await {
        Ok(resp) => HttpResponse::build(resp.status()).body(resp.bytes().await.unwrap()),
        Err(e) => HttpResponse::BadGateway().body(e.to_string()),
    }
}