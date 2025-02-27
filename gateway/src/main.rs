use actix_web::{web, App, HttpRequest,HttpResponse, HttpServer, Responder,Error};
use futures::StreamExt;
use lazy_static::lazy_static;
use reqwest::Client;
use std::collections::HashMap;
use std::net::TcpListener;
use std::process::Stdio;
use std::sync::{
    atomic::{AtomicU16, Ordering},
    Arc, Mutex,
};
use tokio::process::Command as AsyncCommand;
use tokio::time::{sleep, Duration};
use url::Url;

use gateway::{load_config, Function};

lazy_static! {
    static ref SERVICE_REGISTRY: Arc<Mutex<HashMap<String, u16>>> =
        Arc::new(Mutex::new(HashMap::new()));

    static ref SERVICE_CONFIG_MAP: Mutex<HashMap<String, Function>> = {
        Mutex::new(HashMap::new())
    };
}

static NEXT_PORT: AtomicU16 = AtomicU16::new(8050);

// 统一使用 0.0.0.0 检测端口
fn find_available_port(start_port: u16) -> Option<u16> {
    (start_port..8100).find(|port| TcpListener::bind(("0.0.0.0", *port)).is_ok())
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();
    get_config("stack.yaml").await; // 配置文件，但其实不应该在这
    HttpServer::new(move || {
        App::new()
            // .app_data(web::Data::new(config.clone()))
            .route("/{service_name}{path:/?.*}", web::to(handle_request))
    })
    .bind("0.0.0.0:8090")?
    .run()
    .await
}

async fn get_config(config_path: &str) {
    match load_config(config_path) {
        Ok(config) => {
            println!("provider:\n  name: {}", config.provider.name);
            println!("  gateway: {}", config.provider.gateway);
            for (name, function) in &config.functions {
                println!("function: {}", name);
                println!("  lang: {}", function.lang);
                println!("  handler: {}", function.handler);
                println!("  image: {}", function.image);
                println!("  memory: {}", function.memory);
                println!("  environment: {:?}", function.environment);
            }

            let mut functions = SERVICE_CONFIG_MAP.lock().unwrap();
            *functions = config.functions.clone();
        }
        Err(e) => {
            panic!("Failed to load config: {}", e);
        }
    }
}

async fn handle_request(req: HttpRequest, payload: web::Payload) -> impl Responder {
    // let (service_name, rest_path):(String,String) = req.match_info().load().unwrap();
    // let (service_name, rest_path) = (service_name.to_string(), rest_path.to_string());

    let service_name = req.match_info().get("service_name").unwrap().to_string();
    let rest_path = req.match_info().get("path").unwrap_or("").to_string(); 

    println!("{}, {}", service_name, rest_path);

    let port = match get_or_start_service(&service_name).await {
        Ok(p) => p,
        Err(e) => return HttpResponse::InternalServerError().body(e),
    };

    let rest_path = if rest_path.is_empty() {
        "".to_string()
    } else {
        format!("{}", rest_path)
    };

    let target_url = format!("http://0.0.0.0:{}{}", port, rest_path);
    println!("target_url: {}", target_url);

    match proxy_request(&target_url, &req, payload).await{
        Ok(resp)=>{resp}
        Err(e)=>{return HttpResponse::InternalServerError().body(e.to_string())}
    }
}

async fn get_or_start_service(service_name: &str) -> Result<u16, String> {
    let mut registry = SERVICE_REGISTRY.lock().unwrap();

    if let Some(port) = registry.get(service_name) {
        return Ok(*port);
    }

    let start_port = NEXT_PORT.fetch_add(1, Ordering::SeqCst);
    let port = find_available_port(start_port).ok_or("No available ports in range 8050-8100")?;

    println!("🔍 Verifying port {} availability...", port);
    match TcpListener::bind(("0.0.0.0", port)) {
        Ok(_) => println!(" Port {} is available", port),
        Err(e) => return Err(format!("Port {} check failed: {:?}", port, e)),
    }

    println!(" Starting {} service on port {}", service_name, port);
    let mut child = match service_name {
        "docker-echo" |"docker-echo-primes" => {
            let function = SERVICE_CONFIG_MAP.lock().unwrap().get(service_name).cloned();
            let port_forwarding = format!("{}:{}", port, 3000);
            let image_name_str: String = if let Some(func) = function {
                func.image
            } else {
                panic!("Function not found for image in: {}", service_name);
            };
            let mut child = AsyncCommand::new("docker")
                .args(&["run", "-p", &port_forwarding, "--pull=missing", "--rm", "-d", &image_name_str,])
                .stdout(Stdio::piped())
                .spawn()
                .map_err(|e| format!("Failed to start service: {}", e))?;
            let status = child.wait().await.expect("Docker service failed");
            println!("Child exited with status: {}", status);
            child
        }
        "echo" => {
            let child = AsyncCommand::new("cargo")
                .args(&["run", "--bin", "echo", "--", "--port", &port.to_string()])
                .stdout(Stdio::piped())
                .spawn()
                .map_err(|e| format!("Failed to start service: {}", e))?;
            sleep(Duration::from_secs(1)).await;
            println!("Local service started successfully.");
            child
        }
        _ => return Err(format!("Unknown service: {}", service_name)),
    };

    // 等待服务绑定端口
    // sleep(Duration::from_secs(1));
    // let status = child.wait().await.expect("child process failed");
    // println!("Child exited with status: {}", status);

    // 二次验证端口占用
    match TcpListener::bind(("0.0.0.0", port)) {
        Ok(_) => {
            if let Err(kill_error) = child.kill().await {
                return Err(format!("Failed to kill child process: {}", kill_error));
            }
            return Err(format!("Port {} not occupied after startup", port));
        }
        Err(_) => println!(" Port {} occupied successfully", port),
    }

    registry.insert(service_name.to_string(), port);
    Ok(port)
}

async fn proxy_request(target_url: &str, req: &HttpRequest, mut payload: web::Payload) -> Result<HttpResponse,Error>{
    let client = Client::new();
    // let mut url = Url::parse(target_url).unwrap();

    // let service_name = req.match_info().get("service_name").unwrap().to_string();
    // let rest_path = req.match_info().get("path").unwrap_or("").to_string(); 

    // let url=url.join("/{}")

    let mut forwarded_req = client.request(req.method().clone(), target_url)
        .headers(req.headers().clone().into());

    let mut body = web::BytesMut::new();
    while let Some(chunk) = payload.next().await {
        let chunk = chunk?;
        body.extend_from_slice(&chunk);
    }

    let body_bytes = body.freeze();

    if let Some(content_type) = req.headers().get("Content-Type") {
        forwarded_req = forwarded_req.header("Content-Type", content_type);
    }

    match forwarded_req.body(body_bytes).send().await {
         Ok(resp) => //Ok(HttpResponse::build(resp.status())
        //     .insert_header(("Content-Type", resp.headers().get("Content-Type").unwrap()))
        //     .body(resp.bytes().await.unwrap())),
        {
            let status = resp.status();
            let mut client_resp = HttpResponse::build(status);
            
            // 复制所有响应头
            for (name, value) in resp.headers().iter() {
                client_resp.insert_header((name.clone(), value.clone()));
            }
            
            // 读取 body
            let body = resp.bytes().await.unwrap();
            Ok(client_resp.body(body))
        }
        Err(e) => Ok(HttpResponse::BadGateway().body(e.to_string())),
    }
}