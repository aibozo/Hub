use assistant_core::{api, app, config};
use axum::http::{Request, StatusCode};
use axum::Router;
use tower::util::ServiceExt;
use futures_util::StreamExt as _;
use http_body_util::BodyExt as _;

#[tokio::test]
async fn chat_stream_persists_multiple_assistant_turns() {
    // Build app
    let state = app::AppState::new(config::Config::default()).await;
    let app = api::build_router(state.clone());

    // Create session
    let resp = app.clone().oneshot(
        Request::builder().method("POST").uri("/api/chat/sessions").body(axum::body::Body::empty()).unwrap()
    ).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = http_body_util::BodyExt::collect(resp.into_body()).await.unwrap().to_bytes();
    let created: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let sid = created.get("id").and_then(|v| v.as_str()).unwrap().to_string();

    // Helper: append a user message
    async fn append_user(app: &Router, sid: &str, text: &str) {
        let body = serde_json::json!({"role":"user","content": text});
        let resp = app.clone().oneshot(
            Request::builder().method("POST").uri(format!("/api/chat/sessions/{}/append", sid))
                .header("content-type","application/json")
                .body(axum::body::Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap()
        ).await.unwrap();
        assert!(resp.status().is_success());
    }

    // Turn 1: append user and stream mock
    append_user(&app, &sid, "hello").await;
    let req_body = serde_json::json!({
        "session_id": sid,
        "model": "mock",
        "messages": [{"role":"user","content":"hello"}],
        "max_steps": 1
    });
    let resp = app.clone().oneshot(
        Request::builder().method("POST").uri("/api/chat/stream")
            .header("content-type","application/json")
            .body(axum::body::Body::from(serde_json::to_vec(&req_body).unwrap())).unwrap()
    ).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let mut stream = resp.into_body().into_data_stream();
    let mut buf = String::new();
    // read until done
    for _ in 0..20 {
        if let Some(Ok(bytes)) = stream.next().await {
            buf.push_str(&String::from_utf8_lossy(&bytes));
            if buf.contains("event: done") { break; }
        } else { break; }
    }

    // Turn 2: another user + stream
    append_user(&app, &sid, "second").await;
    let req_body2 = serde_json::json!({
        "session_id": sid,
        "model": "mock",
        "messages": [{"role":"user","content":"second"}],
        "max_steps": 1
    });
    let resp2 = app.clone().oneshot(
        Request::builder().method("POST").uri("/api/chat/stream")
            .header("content-type","application/json")
            .body(axum::body::Body::from(serde_json::to_vec(&req_body2).unwrap())).unwrap()
    ).await.unwrap();
    assert_eq!(resp2.status(), StatusCode::OK);
    let mut stream2 = resp2.into_body().into_data_stream();
    let mut buf2 = String::new();
    for _ in 0..20 {
        if let Some(Ok(bytes)) = stream2.next().await {
            buf2.push_str(&String::from_utf8_lossy(&bytes));
            if buf2.contains("event: done") { break; }
        } else { break; }
    }

    // Get session and assert two assistant messages exist
    let resp3 = app.clone().oneshot(
        Request::builder().method("GET").uri(format!("/api/chat/sessions/{}", sid))
            .body(axum::body::Body::empty()).unwrap()
    ).await.unwrap();
    assert_eq!(resp3.status(), StatusCode::OK);
    let bytes3 = http_body_util::BodyExt::collect(resp3.into_body()).await.unwrap().to_bytes();
    let sess: serde_json::Value = serde_json::from_slice(&bytes3).unwrap();
    let msgs = sess.get("messages").and_then(|v| v.as_array()).unwrap();
    let assistants: Vec<&serde_json::Value> = msgs.iter().filter(|m| m.get("role").and_then(|r| r.as_str()) == Some("assistant")).collect();
    assert!(assistants.len() >= 2, "expected at least 2 assistant messages, got {} (full: {:?})", assistants.len(), msgs);
    for a in assistants.iter() {
        let content = a.get("content").and_then(|c| c.as_str()).unwrap_or("");
        assert!(!content.is_empty(), "assistant content should not be empty");
    }
}
