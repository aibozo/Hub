use assistant_core::{api, app, config};
use axum::body::to_bytes;
use axum::http::{Request, StatusCode};
use tower::util::ServiceExt;

#[tokio::test]
async fn agent_replan_writes_artifact_and_updates_row() {
    let state = app::AppState::new(config::Config::default()).await;
    let app = api::build_router(state.clone());
    let mem = state.handles.memory.as_ref().expect("memory");

    // Prepare task + agent
    let task = mem.store.create_task("Replan", "open", None).await.unwrap();
    let agent_id = format!("agent-{}", uuid::Uuid::new_v4());
    let _ = mem
        .store
        .create_agent(&agent_id, task.id, "A", "Draft", "dev/replan_test", None, None, 1, None)
        .await
        .unwrap();

    // Replan with inline content
    let plan_md = "# Plan\n- step 1";
    let body = serde_json::json!({"content_md": plan_md});
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/agents/{}/replan", agent_id))
                .header("content-type", "application/json")
                .body(axum::body::Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // Verify Agent.plan_artifact_id set and file exists
    let storage_root = state.handles.system_map.map_path().parent().unwrap().to_path_buf();
    let plan_path = storage_root.join("agents").join(&agent_id).join("plan.md");
    let content = std::fs::read_to_string(&plan_path).expect("plan file");
    assert!(content.contains("# Plan"));

    let agent = mem.store.get_agent(&agent_id).await.unwrap().unwrap();
    let plan_id = agent.plan_artifact_id.expect("plan id");

    // Event recorded
    let evs = mem.store.get_recent_events_by_agent(&agent_id, 10).await.unwrap();
    assert!(evs.iter().any(|e| e.kind == "agent.replan"));

    // Artifacts API returns our artifact
    let resp2 = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/agents/{}/artifacts", agent_id))
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp2.status(), StatusCode::OK);
    let b = to_bytes(resp2.into_body(), 64 * 1024).await.unwrap();
    let arts: Vec<serde_json::Value> = serde_json::from_slice(&b).unwrap();
    assert!(arts.iter().any(|a| a.get("id").and_then(|v| v.as_i64()) == Some(plan_id)));
}

#[tokio::test]
async fn agent_resume_via_api_runs_ctr_with_auto_approval() {
    let state = app::AppState::new(config::Config::default()).await;
    let app = api::build_router(state.clone());
    let mem = state.handles.memory.as_ref().expect("memory");

    let task = mem.store.create_task("Resume", "open", None).await.unwrap();
    let root_rel = format!("dev/resume_{}", uuid::Uuid::new_v4());
    let agent_id = format!("agent-{}", uuid::Uuid::new_v4());
    let _ = mem
        .store
        .create_agent(&agent_id, task.id, "R", "Paused", &root_rel, None, None, 2, None)
        .await
        .unwrap();

    // Resume via API
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/agents/{}/resume", agent_id))
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // Wait until Done or until the file exists (CTR apply succeeded).
    let storage_root = state.handles.system_map.map_path().parent().unwrap().to_path_buf();
    let path = storage_root.join(&root_rel).join("CTR_HELLO.txt");
    let mut tries = 0;
    let mut file_ok = false;
    let mut status_done = false;
    loop {
        let a = mem.store.get_agent(&agent_id).await.unwrap().unwrap();
        status_done = a.status == "Done";
        if status_done { break; }
        if std::fs::metadata(&path).is_ok() { file_ok = true; break; }
        tries += 1;
        if tries > 300 { break; } // ~6s
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    assert!(status_done || file_ok, "CTR did not complete; file_ok={} status_done={}", file_ok, status_done);
}
