use actix_web::{http::header, test, web, App, HttpRequest, Responder};
use serde_json::json;

use crate::reverse_proxy::ReverseProxy;

async fn proxy_handler(
    req: HttpRequest,
    payload: web::Payload,
    proxy: web::Data<ReverseProxy>,
) -> impl Responder {
    proxy.handle_request(req, payload).await
}

#[actix_web::test]
async fn echo_test() {
    let proxy = web::Data::new(ReverseProxy::new());

    let app = test::init_service(
        App::new()
            .app_data(proxy.clone())
            .route("/{service_name}{path:/?.*}", web::to(proxy_handler)),
    )
    .await;

    let req = test::TestRequest::post()
        .uri("/echo/echo")
        .set_json(json!({"message":"Hello"}))
        .insert_header(("Content-Type", "application/json"))
        .to_request();

    let resp = test::call_service(&app, req).await;
    dbg!(&resp);

    let req_headers = resp.request().headers().iter();

    println!("response status:{}", resp.status());
    assert!(resp.status().is_success());

    for (key, value) in req_headers {
        assert_eq!(
            resp.headers().get(key),
            Some(value),
            "Response header {:?} does not match request header",
            key
        );
    }
}
