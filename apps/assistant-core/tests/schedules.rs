use assistant_core::{api, app, config};
use axum::{http::Request, body::Body};
use tower::ServiceExt;

#[tokio::test]
async fn schedules_endpoint_returns_jobs() {
    let state = app::AppState::new(config::Config::default()).await;
    let app_router = api::build_router(state);
    let resp = app_router
        .oneshot(Request::builder().uri("/api/schedules").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert!(resp.status().is_success());
}

