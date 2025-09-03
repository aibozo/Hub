use assistant_core::{api, app, config};
use axum::{http::Request, body::{Body, to_bytes}};
use tower::ServiceExt;

#[tokio::test]
async fn arxiv_search_and_top_stub() {
    // Use default config; arxiv manifest without stdio transport falls back to in-core stub.
    let state = app::AppState::new(config::Config::default()).await;
    let app_router = api::build_router(state);

    // search
    let body = serde_json::json!({"params": {"query": "mixture-of-experts"}}).to_string();
    let resp = app_router.clone().oneshot(
        Request::builder()
            .method("POST").uri("/api/tools/arxiv/search")
            .header("content-type","application/json")
            .body(Body::from(body)).unwrap()
    ).await.unwrap();
    assert!(resp.status().is_success());
    let bytes = to_bytes(resp.into_body(), 1024 * 1024).await.unwrap();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert!(v.get("results").and_then(|x| x.as_array()).is_some());

    // top
    let body = serde_json::json!({"params": {"month": "2025-01", "n": 3}}).to_string();
    let resp = app_router.clone().oneshot(
        Request::builder()
            .method("POST").uri("/api/tools/arxiv/top")
            .header("content-type","application/json")
            .body(Body::from(body)).unwrap()
    ).await.unwrap();
    assert!(resp.status().is_success());
    let bytes = to_bytes(resp.into_body(), 1024 * 1024).await.unwrap();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let items = v.get("items").and_then(|x| x.as_array()).unwrap();
    assert_eq!(items.len(), 3);
}

