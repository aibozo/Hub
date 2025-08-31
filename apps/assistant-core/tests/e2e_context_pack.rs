use assistant_core::{api, app, config};
use axum::{http::Request, body::{Body, to_bytes}};
use tower::ServiceExt;

#[tokio::test]
async fn context_pack_includes_system_digest() {
    let state = app::AppState::new(config::Config::default()).await;
    let app_router = api::build_router(state);
    let req = Request::builder()
        .method("POST")
        .uri("/api/context/pack")
        .header("content-type","application/json")
        .body(Body::from("{}"))
        .unwrap();
    let resp = app_router.clone().oneshot(req).await.unwrap();
    assert!(resp.status().is_success());
    let b = to_bytes(resp.into_body(), 1024*1024).await.unwrap();
    let v: serde_json::Value = serde_json::from_slice(&b).unwrap();
    assert!(v.get("system_digest").and_then(|s| s.as_str()).map(|s| !s.is_empty()).unwrap_or(false));
}

