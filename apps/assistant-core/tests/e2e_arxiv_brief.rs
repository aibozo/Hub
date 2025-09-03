use assistant_core::{api, app, config};
use axum::{http::Request, body::Body};
use tower::ServiceExt;

#[tokio::test]
async fn arxiv_brief_writes_md_and_json() {
    // Enable multiagent optionally to exercise the path
    std::env::set_var("RESEARCH_MULTIAGENT", "1");
    let state = app::AppState::new(config::Config::default()).await;
    let app_router = api::build_router(state);

    let resp = app_router.clone().oneshot(
        Request::builder().method("POST").uri("/api/schedules/run/arxiv").body(Body::empty()).unwrap()
    ).await.unwrap();
    assert!(resp.status().is_success());

    let date = chrono::Local::now().format("%Y-%m-%d").to_string();
    let base = std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../"));
    let md = base.join(format!("storage/briefs/{}-arxiv.md", date));
    let js = base.join(format!("storage/briefs/{}-arxiv.json", date));
    assert!(md.exists(), "expected markdown {}", md.display());
    assert!(js.exists(), "expected json {}", js.display());
    let text = std::fs::read_to_string(&md).unwrap_or_default();
    assert!(text.contains("arXiv Brief"));
    let v: serde_json::Value = serde_json::from_slice(&std::fs::read(&js).unwrap()).unwrap();
    assert_eq!(v.get("kind").and_then(|x| x.as_str()), Some("research_report/v1"));
}
