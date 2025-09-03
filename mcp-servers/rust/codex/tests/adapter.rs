use std::io::{BufRead, Write};
use std::process::{Command, Stdio};

#[test]
fn codex_bridge_captures_session_id() {
    // Resolve binaries
    let exe = std::env::var("CARGO_BIN_EXE_mcp-codex").unwrap_or_else(|_| "./target/debug/mcp-codex".into());
    let codex_mock = std::env::var("CARGO_BIN_EXE_codex-mock").unwrap_or_else(|_| "./target/debug/codex-mock".into());

    // Spawn mcp-codex with CODEX_BIN pointed to codex-mock
    let mut child = Command::new(exe)
        .env("CODEX_BIN", &codex_mock)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("spawn mcp-codex");

    // Send a ToolRequest line: { tool: "new", params: { prompt: "hi" } }
    let req = foreman_mcp::ToolRequest { tool: "new".to_string(), params: serde_json::json!({"prompt":"test"}) };
    let line = serde_json::to_string(&req).expect("json");
    child.stdin.as_mut().unwrap().write_all(format!("{}\n", line).as_bytes()).expect("write");

    // Read lines until we get a ToolResponse (notifications may appear first)
    let stdout = child.stdout.take().unwrap();
    let mut reader = std::io::BufReader::new(stdout).lines();
    let mut got_resp = None;
    for _ in 0..10 {
        if let Some(Ok(line)) = reader.next() {
            if let Ok(resp) = serde_json::from_str::<foreman_mcp::ToolResponse>(&line) {
                got_resp = Some(resp);
                break;
            }
            // else: ignore notifications
        }
    }
    let resp = got_resp.expect("tool response");
    assert!(resp.ok, "ok");
    let sid = resp.result.get("session_id").and_then(|v| v.as_str()).unwrap_or("");
    assert_eq!(sid, "mock-123", "session id captured from notification");
}

