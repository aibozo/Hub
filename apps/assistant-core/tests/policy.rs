use assistant_core::{api, app, gatekeeper::ProposedAction};
use axum::http::{Request, StatusCode};
use tower::util::ServiceExt;

#[tokio::test]
async fn policy_check_allows_readonly() {
    let state = app::AppState::new(Default::default()).await;
    let app = api::build_router(state);
    let payload = serde_json::json!({
        "command": "cat /etc/hosts",
        "writes": false,
        "paths": ["/etc/hosts"],
    });
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/policy/check")
                .header("content-type", "application/json")
                .body(axum::body::Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}
