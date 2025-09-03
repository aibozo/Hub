use assistant_core::{api, app, config};
use axum::body::to_bytes;
use axum::http::{Request, StatusCode};
use tower::util::ServiceExt;

#[tokio::test]
async fn codex_continue_logs_prompt_even_on_failure() {
    // Build app with default state; no Codex MCP server is expected to be running.
    let state = app::AppState::new(config::Config::default()).await;
    let app = api::build_router(state.clone());

    let body = serde_json::json!({
        "session_id": "sess-1",
        "prompt": "Hello Codex"
    });

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/codex/continue")
                .header("content-type", "application/json")
                .body(axum::body::Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Without a running Codex MCP server, we expect a 502 Bad Gateway.
    assert_eq!(resp.status(), StatusCode::BAD_GATEWAY);

    // But the prompt event should have been recorded regardless.
    let mem = state.handles.memory.as_ref().expect("memory");
    let evs = mem.store.get_recent_events(50).await.expect("events");
    assert!(evs.iter().any(|e| {
        if e.kind != "mcp_codex_prompt" { return false; }
        if let Some(payload) = e.payload_json.as_ref().and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok()) {
            payload.get("session_id").and_then(|v| v.as_str()) == Some("sess-1")
                && payload.get("text").and_then(|v| v.as_str()) == Some("Hello Codex")
        } else { false }
    }));
}

#[tokio::test]
async fn codex_sessions_and_detail_aggregate_from_memory() {
    let state = app::AppState::new(config::Config::default()).await;
    let app = api::build_router(state.clone());
    let mem = state.handles.memory.as_ref().expect("memory");

    // Seed events for a synthetic session.
    let sid = "sess-xyz";
    let _ = mem
        .store
        .append_event(
            None,
            "mcp_codex_session_configured",
            Some(&serde_json::json!({ "session_id": sid, "source": "test", "raw": {"ok":true} })),
        )
        .await
        .expect("seed session");
    let _ = mem
        .store
        .append_event(
            None,
            "mcp_codex_prompt",
            Some(&serde_json::json!({ "session_id": sid, "text": "First prompt" })),
        )
        .await
        .expect("seed prompt");
    let _ = mem
        .store
        .append_event(
            None,
            "mcp_codex_continue",
            Some(&serde_json::json!({ "session_id": sid, "result": {"message": "ok"} })),
        )
        .await
        .expect("seed continue");

    // List sessions
    let resp = app
        .clone()
        .oneshot(Request::builder().method("GET").uri("/api/codex/sessions").body(axum::body::Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = to_bytes(resp.into_body(), 64 * 1024).await.unwrap();
    let rows: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();
    assert!(rows.iter().any(|r| r.get("session_id").and_then(|v| v.as_str()) == Some(sid)));

    // Session detail
    let resp2 = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/codex/session/{}", sid))
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp2.status(), StatusCode::OK);
    let body2 = to_bytes(resp2.into_body(), 256 * 1024).await.unwrap();
    let detail: serde_json::Value = serde_json::from_slice(&body2).unwrap();
    let text = detail.get("text").and_then(|v| v.as_str()).unwrap_or("");
    assert!(text.contains("session started"));
    assert!(text.contains("you: First prompt"));
    assert!(text.contains("continue:"));
}
