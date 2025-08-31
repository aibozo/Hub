#![cfg(feature = "realtime")]

use assistant_core::realtime::{RealtimeManager, RealtimeOptions};
use axum::{routing::get, Router};
use axum::extract::ws::{WebSocketUpgrade, Message, WebSocket};
use axum::response::IntoResponse;
use futures_util::{StreamExt, SinkExt};

async fn ws_handler(ws: WebSocketUpgrade) -> impl IntoResponse { ws.on_upgrade(handle_ws) }

async fn handle_ws(mut socket: WebSocket) {
    // Expect a session.update; echo session.updated
    if let Some(Ok(Message::Text(_t))) = socket.recv().await {
        let _ = socket.send(Message::Text("{\"type\":\"session.updated\"}".into())).await;
        // Send a tiny audio delta event to exercise the client path (ignored if audio feature is off)
        let audio_delta = serde_json::json!({
            "type": "response.output_audio.delta",
            "audio": ""
        });
        let _ = socket.send(Message::Text(audio_delta.to_string())).await;
        // Now request a simple tool call
        let call = serde_json::json!({
            "type": "tool.call",
            "id": "1",
            "name": "shell.which",
            "arguments": {"cmd": "echo"}
        });
        let _ = socket.send(Message::Text(call.to_string())).await;
        // Expect a tool.output
        if let Some(Ok(Message::Text(txt))) = socket.recv().await {
            let v: serde_json::Value = serde_json::from_str(&txt).unwrap_or_else(|_| serde_json::json!({}));
            assert_eq!(v.get("type").and_then(|s| s.as_str()), Some("tool.output"));
            assert_eq!(v.get("id").and_then(|s| s.as_str()), Some("1"));
        }
    }
    let _ = socket.send(Message::Close(None)).await;
}

#[tokio::test]
async fn realtime_connects_to_mock() {
    // Start mock WS server on random port
    let app = Router::new().route("/realtime", get(ws_handler));
    let listener = tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0)).await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap(); });

    // Start the realtime manager pointing to the mock endpoint
    let rt = RealtimeManager::new();
    let endpoint = format!("ws://{}:{}/realtime", addr.ip(), addr.port());
    rt.start(RealtimeOptions { model: Some("gpt-realtime".into()), voice: None, audio: None, instructions: None, endpoint: Some(endpoint) }).await.unwrap();

    // Wait briefly for state to flip to active and to process one tool call
    tokio::time::sleep(std::time::Duration::from_millis(80)).await;
    let st = rt.status();
    assert!(st.active, "status should be active after connect");

    // Request stop and verify inactive
    rt.stop().await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    let st2 = rt.status();
    assert!(!st2.active, "status should be inactive after stop");
}
