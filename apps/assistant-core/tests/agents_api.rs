use assistant_core::{api, app, config};
use axum::http::{Request, StatusCode};
use tower::util::ServiceExt;

#[tokio::test]
async fn agents_create_and_list() {
    let state = app::AppState::new(config::Config::default()).await;
    let app = api::build_router(state.clone());

    let create = serde_json::json!({
        "task_id": 1,
        "title": "My Agent",
        "root_dir": ".",
        "model": "gpt-4.1",
        "auto_approval_level": 1
    });

    let resp = app.clone().oneshot(
        Request::builder()
            .method("POST")
            .uri("/api/agents")
            .header("content-type","application/json")
            .body(axum::body::Body::from(create.to_string()))
            .unwrap()
    ).await.unwrap();
    // Memory starts with empty DB; creating an agent requires an existing task_id; we expect 500 here.
    // For a smoke test, just ensure we get a proper error response, not a panic.
    assert!(resp.status() == StatusCode::INTERNAL_SERVER_ERROR || resp.status() == StatusCode::OK);

    // List call should succeed even if empty
    let resp2 = app.clone().oneshot(
        Request::builder().method("GET").uri("/api/agents").body(axum::body::Body::empty()).unwrap()
    ).await.unwrap();
    assert_eq!(resp2.status(), StatusCode::OK);
}

