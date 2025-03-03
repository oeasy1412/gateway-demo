use actix_web::{web, Error, HttpRequest, HttpResponse, Responder};

use futures::StreamExt;
use reqwest::Client;
use std::net::IpAddr;
use std::net::Ipv4Addr;
use std::net::TcpListener;
use std::process::Stdio;
use std::sync::atomic::{AtomicU16, Ordering};

use tokio::process::Command as AsyncCommand;
use tokio::time::{sleep, Duration};
use url::Url;

use crate::registry::Registry;

static NEXT_PORT: AtomicU16 = AtomicU16::new(8050);

pub struct ReverseProxy {
    pub client: Client,
    pub registry: Registry,
}

impl ReverseProxy {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            registry: Registry::new(),
        }
    }

    fn find_available_port(&self, start_port: u16) -> Option<u16> {
        (start_port..8100).find(|port| TcpListener::bind(("0.0.0.0", *port)).is_ok())
    }

    pub async fn handle_request(&self, req: HttpRequest, payload: web::Payload) -> impl Responder {
        let service_name = req.match_info().get("service_name").unwrap();
        let rest_path = req.match_info().get("path").unwrap_or("");
        println!("111");

        let (address, port) = match self.get_or_start_service(service_name).await {
            Ok((a, p)) => (a, p),
            Err(e) => return HttpResponse::InternalServerError().body(e),
        };

        println!("Service Name: {}, Rest Path: {}", service_name, rest_path);

        let user_input = format!("http://{}:{}{}", address, port, rest_path);
        let target_url = match Url::parse(&user_input) {
            Ok(mut url) => {
                url.set_path(&url.path().replace("//", "/"));
                // url.path_segments_mut().map_err(|_| "cannot be base").unwrap().pop();
                url
            }
            Err(_) => {
                eprint!("无法解析 URL,请检查格式是否正确");
                Url::parse("").unwrap()
            }
        };
        println!("target_url: {}", target_url);
        let url_str = target_url.to_string();

        match self.proxy_request(&url_str, &req, payload).await {
            Ok(resp) => resp,
            Err(e) => return HttpResponse::InternalServerError().body(e.to_string()),
        }
    }

    async fn get_or_start_service(&self, service_name: &str) -> Result<(IpAddr, u16), String> {
        let mut port: u16 = Default::default();
        let address = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
        if Registry::check_service_config(service_name) || service_name == "echo" {
            if let Some((address, port)) = self.registry.get_service_endpoint(service_name) {
                return Ok((address, port));
            }

            let start_port = NEXT_PORT.fetch_add(1, Ordering::SeqCst);
            port = self
                .find_available_port(start_port)
                .ok_or("No available ports in range 8050-8100")?;

            println!(" Verifying port {} availability...", port);
            match TcpListener::bind((address, port)) {
                Ok(_) => println!(" Port {} is available", port),
                Err(e) => return Err(format!("Port {} check failed: {:?}", port, e)),
            }

            println!("Starting {} service on port {}", service_name, port);
            let endpoint = (address, port);
            self.registry.insert_service(service_name, endpoint);
        }
        let mut child = match service_name {
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
            _ => {
                if Registry::check_service_config(service_name) {
                    let function = Registry::get_service_from_config(service_name);
                    let port_forwarding = format!("{}:{}", port, 3000);
                    let image_name_str: String = if let Some(func) = function {
                        func.image
                    } else {
                        panic!("Function not found for image in: {}", service_name);
                    };
                    let mut child = AsyncCommand::new("docker")
                        .args(&[
                            "run",
                            "-p",
                            &port_forwarding,
                            "--pull=missing",
                            "--rm",
                            "-d",
                            &image_name_str,
                        ])
                        .stdout(Stdio::piped())
                        .spawn()
                        .map_err(|e| format!("Failed to start service: {}", e))?;
                    let status = child.wait().await.expect("Docker service failed");
                    println!("Child exited with status: {}", status);
                    child
                } else {
                    return Err(format!("Unknown service: {}", service_name));
                }
            }
        };

        match TcpListener::bind((address, port)) {
            Ok(_) => {
                if let Err(kill_error) = child.kill().await {
                    return Err(format!("Failed to kill child process: {}", kill_error));
                }
                return Err(format!("Port {} not occupied after startup", port));
            }
            Err(_) => println!(" Port {} occupied successfully", port),
        }
        Ok((address,port))
    }


    async fn proxy_request(
        &self,
        target_url: &str,
        req: &HttpRequest,
        mut payload: web::Payload,
    ) -> Result<HttpResponse, Error> {
        let  forwarded_req = self
            .client
            .request(req.method().clone(), target_url)
            .headers(req.headers().clone().into());
        let mut body = web::BytesMut::new();
        while let Some(chunk) = payload.next().await {
            let chunk = chunk?;
            body.extend_from_slice(&chunk);
        }

        let body_bytes = body.freeze();

        match forwarded_req.body(body_bytes).send().await {
            Ok(resp) =>
            //Ok(HttpResponse::build(resp.status())
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
}
