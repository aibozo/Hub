use assistant_core::{api, app, config};
use axum::http::{Request, Method, header};
use tower::util::ServiceExt;

#[tokio::test]
async fn cors_preflight_chat_stream_allows_localhost_3000() {
    let state = app::AppState::new(config::Config::default()).await;
    let app = api::build_router(state);

    let req = Request::builder()
        .method(Method::OPTIONS)
        .uri("/api/chat/stream")
        .header(header::ORIGIN, "http://localhost:3000")
        .header(header::ACCESS_CONTROL_REQUEST_METHOD, "POST")
        .header(header::ACCESS_CONTROL_REQUEST_HEADERS, "content-type")
        .body(axum::body::Body::empty())
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert!(resp.status().is_success());
    let allow_origin = resp.headers().get(header::ACCESS_CONTROL_ALLOW_ORIGIN).and_then(|v| v.to_str().ok()).unwrap_or("");
    assert_eq!(allow_origin, "http://localhost:3000");
    let allow_methods = resp.headers().get(header::ACCESS_CONTROL_ALLOW_METHODS).and_then(|v| v.to_str().ok()).unwrap_or("");
    assert!(allow_methods.contains("POST"));
    let allow_headers = resp.headers().get(header::ACCESS_CONTROL_ALLOW_HEADERS).and_then(|v| v.to_str().ok()).unwrap_or("");
    assert!(allow_headers.to_ascii_lowercase().contains("content-type"));
}

#[tokio::test]
async fn cors_get_health_includes_allow_origin() {
    let state = app::AppState::new(config::Config::default()).await;
    let app = api::build_router(state);

    let req = Request::builder()
        .method(Method::GET)
        .uri("/health")
        .header(header::ORIGIN, "http://127.0.0.1:3000")
        .body(axum::body::Body::empty())
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert!(resp.status().is_success());
    let allow_origin = resp.headers().get(header::ACCESS_CONTROL_ALLOW_ORIGIN).and_then(|v| v.to_str().ok()).unwrap_or("");
    assert_eq!(allow_origin, "http://127.0.0.1:3000");
}

