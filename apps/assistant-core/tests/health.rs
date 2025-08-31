use assistant_core::{api, app, config};
use axum::http::{Request, StatusCode};
use tower::util::ServiceExt; // for `oneshot`

#[tokio::test]
async fn health_endpoint_returns_ok() {
    let state = app::AppState::new(config::Config::default()).await;
    let app = api::build_router(state);

    let response = app
        .oneshot(Request::builder().uri("/health").body(axum::body::Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}
