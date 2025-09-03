use assistant_core::{api, app, config};
use axum::{http::Request, body::{Body, to_bytes}};
use tower::ServiceExt;
use axum::http::StatusCode;

#[tokio::test]
async fn shell_list_and_read() {
    // Create temp dir and file
    // Create temp dir under system temp
    let tmp_dir = std::env::temp_dir().join(format!("foreman_test_{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&tmp_dir).unwrap();
    let file_path = tmp_dir.join("hello.txt");
    std::fs::write(&file_path, "world").unwrap();

    let state = app::AppState::new(config::Config::default()).await;
    let app_router = api::build_router(state);

    // list_dir
    let body = serde_json::json!({"params": {"path": tmp_dir.to_string_lossy()}}).to_string();
    let resp = app_router.clone().oneshot(
        Request::builder()
            .method("POST").uri("/api/tools/shell/list_dir")
            .header("content-type","application/json")
            .body(Body::from(body)).unwrap()
    ).await.unwrap();
    assert!(resp.status().is_success());
    let bytes = to_bytes(resp.into_body(), 1024 * 1024).await.unwrap();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert!(v.get("entries").is_some());

    // read_file
    let body = serde_json::json!({"params": {"path": file_path.to_string_lossy()}}).to_string();
    let resp = app_router.clone().oneshot(
        Request::builder()
            .method("POST").uri("/api/tools/shell/read_file")
            .header("content-type","application/json")
            .body(Body::from(body)).unwrap()
    ).await.unwrap();
    assert!(resp.status().is_success());
    let bytes = to_bytes(resp.into_body(), 1024 * 1024).await.unwrap();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(v.get("content").and_then(|s| s.as_str()), Some("world"));
}

#[tokio::test]
async fn patch_apply_requires_approval() {
    let state = app::AppState::new(config::Config::default()).await;
    let app_router = api::build_router(state);
    let body = serde_json::json!({"params": {"edits": [{"path": "./tmp.txt", "content": "hello"}]}}).to_string();
    let resp = app_router.clone().oneshot(
        Request::builder().method("POST").uri("/api/tools/patch/apply").header("content-type","application/json").body(Body::from(body)).unwrap()
    ).await.unwrap();
    assert_eq!(resp.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn git_commit_requires_approval() {
    let state = app::AppState::new(config::Config::default()).await;
    let app_router = api::build_router(state);
    let body = serde_json::json!({"params": {"path": ".", "message": "test"}}).to_string();
    let resp = app_router.clone().oneshot(
        Request::builder().method("POST").uri("/api/tools/git/commit").header("content-type","application/json").body(Body::from(body)).unwrap()
    ).await.unwrap();
    assert_eq!(resp.status(), StatusCode::CONFLICT);
}
