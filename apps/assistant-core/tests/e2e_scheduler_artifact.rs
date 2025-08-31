use assistant_core::{api, app, config};
use axum::{http::Request, body::Body};
use tower::ServiceExt;

#[tokio::test]
async fn scheduler_run_logs_event() {
    let home = format!("./storage/test_home_{}", uuid::Uuid::new_v4());
    let cfg = config::Config { foreman: Some(config::ForemanConfig { home: Some(home.clone()), profile: None }), ..Default::default() };
    let state = app::AppState::new(cfg).await;
    let app_router = api::build_router(state.clone());

    let resp = app_router.clone().oneshot(
        Request::builder().method("POST").uri("/api/schedules/run/arxiv").body(Body::empty()).unwrap()
    ).await.unwrap();
    assert!(resp.status().is_success());

    if let Some(mem) = state.handles.memory.as_ref() {
        let events = mem.store.get_recent_events(50).await.unwrap();
        assert!(events.iter().any(|e| e.kind == "scheduler:arxiv:run"));
    }
}

