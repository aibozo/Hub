use assistant_core::{api, app, config};
use axum::http::Request;
use tower::util::ServiceExt;
use std::time::Duration;

#[tokio::test]
async fn agents_sse_streams_backlog_and_live_events() {
    let state = app::AppState::new(config::Config::default()).await;
    let app_router = api::build_router(state.clone());
    let mem = state.handles.memory.as_ref().expect("memory");

    // Create task + agent and seed a backlog event
    let task = mem.store.create_task("SSE", "open", None).await.unwrap();
    let agent_id = format!("agent-{}", uuid::Uuid::new_v4());
    let _ = mem
        .store
        .create_agent(&agent_id, task.id, "A", "Running", "dev/sse_test", None, None, 1, None)
        .await
        .unwrap();
    let _ = mem
        .store
        .append_event_for_agent(Some(task.id), Some(&agent_id), "agent.done", None)
        .await
        .unwrap();

    // Start SSE stream
    let resp = app_router
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/agents/{}/events", agent_id))
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), axum::http::StatusCode::OK);
    let ctype = resp.headers().get(axum::http::header::CONTENT_TYPE).unwrap().to_str().unwrap();
    assert!(ctype.starts_with("text/event-stream"));

    // Convert body to data stream; read a few chunks
    use http_body_util::BodyExt as _;
    use futures_util::StreamExt as _;
    let mut stream = resp.into_body().into_data_stream();

    // Emit a live event after the stream starts
    let mem2 = state.handles.memory.as_ref().unwrap().clone();
    let agent2 = agent_id.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(200)).await;
        let _ = mem2
            .store
            .append_event_for_agent(None, Some(&agent2), "agent.paused", None)
            .await;
    });

    let mut buf = String::new();
    let mut saw_backlog = false;
    let mut saw_live = false;
    for _ in 0..20 {
        if let Some(Ok(bytes)) = stream.next().await {
            let chunk = String::from_utf8_lossy(&bytes);
            buf.push_str(&chunk);
            if !saw_backlog && buf.contains("event: status") && buf.contains("agent.done") {
                saw_backlog = true;
            }
            if !saw_live && buf.contains("agent.paused") {
                saw_live = true;
                break;
            }
        } else {
            break;
        }
    }
    assert!(saw_backlog, "did not observe backlog status event with agent.done; got: {}", buf);
    assert!(saw_live, "did not observe live event agent.paused; got: {}", buf);
}

#[tokio::test]
async fn agents_pause_and_abort_endpoints() {
    let state = app::AppState::new(config::Config::default()).await;
    let app_router = api::build_router(state.clone());
    let mem = state.handles.memory.as_ref().expect("memory");

    let task = mem.store.create_task("PA", "open", None).await.unwrap();
    let agent_id = format!("agent-{}", uuid::Uuid::new_v4());
    let _ = mem
        .store
        .create_agent(&agent_id, task.id, "A", "Running", "dev/pa_test", None, None, 1, None)
        .await
        .unwrap();

    // Pause
    let resp = app_router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/agents/{}/pause", agent_id))
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), axum::http::StatusCode::OK);
    let a = mem.store.get_agent(&agent_id).await.unwrap().unwrap();
    assert_eq!(a.status, "Paused");

    // Abort
    let resp2 = app_router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/agents/{}/abort", agent_id))
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp2.status(), axum::http::StatusCode::OK);
    let a2 = mem.store.get_agent(&agent_id).await.unwrap().unwrap();
    assert_eq!(a2.status, "Aborted");
}

