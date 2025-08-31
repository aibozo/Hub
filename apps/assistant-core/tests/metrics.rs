use assistant_core::{api, app, config};
use axum::{http::Request, body::Body};
use tower::ServiceExt;

#[tokio::test]
async fn metrics_increment_on_health() {
    let state = app::AppState::new(config::Config::default()).await;
    let app_router = api::build_router(state);

    // Hit /health
    let _ = app_router.clone().oneshot(Request::builder().uri("/health").body(Body::empty()).unwrap()).await.unwrap();

    // Fetch metrics
    let resp = app_router
        .clone()
        .oneshot(Request::builder().uri("/metrics").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert!(resp.status().is_success());
    let bytes = axum::body::to_bytes(resp.into_body(), 1024 * 1024).await.unwrap();
    let s = String::from_utf8(bytes.to_vec()).unwrap();
    assert!(s.contains("foreman_api_requests_total"), "metrics should include api counter");
}

