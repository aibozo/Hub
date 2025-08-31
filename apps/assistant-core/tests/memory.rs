use assistant_core::{api, app, config};
use axum::http::{Request, StatusCode};
use tower::util::ServiceExt;

#[tokio::test]
async fn create_and_list_tasks() {
    let state = app::AppState::new(config::Config::default()).await;
    let app = api::build_router(state);

    // Create
    let create = serde_json::json!({"title":"Test","status":"open"});
    let resp = app.clone().oneshot(
        Request::builder().method("POST").uri("/api/tasks").header("content-type","application/json")
        .body(axum::body::Body::from(create.to_string())).unwrap()
    ).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // List
    let resp = app.oneshot(Request::builder().uri("/api/tasks").body(axum::body::Body::empty()).unwrap()).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

