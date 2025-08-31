use assistant_core::{api, app, config};
use axum::{http::Request, body::{Body, to_bytes}};
use tower::ServiceExt;

#[tokio::test]
async fn project_init_creates_dir_under_home() {
    // Set a temp HOME
    let tmp = tempfile::tempdir().unwrap();
    std::env::set_var("HOME", tmp.path());
    // Ensure roots exist
    std::fs::create_dir_all(tmp.path().join("dev")).unwrap();
    // Build app and router
    let state = app::AppState::new(config::Config::default()).await;
    let app_router = api::build_router(state.clone());
    let params = serde_json::json!({"params": {"name": "demo", "kind": "dev", "git": false}});
    let req = Request::builder()
        .method("POST").uri("/api/tools/project/init")
        .header("content-type","application/json")
        .body(Body::from(params.to_string())).unwrap();
    let resp = app_router.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), axum::http::StatusCode::OK);
    let body = to_bytes(resp.into_body(), 1024*1024).await.unwrap();
    let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let path = v.get("path").and_then(|x| x.as_str()).unwrap();
    assert!(std::path::Path::new(path).exists());
}
