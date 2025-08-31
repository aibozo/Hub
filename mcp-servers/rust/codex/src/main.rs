use anyhow::{anyhow, Context, Result};
use foreman_mcp::{ToolRequest, ToolResponse};
use serde_json::{json, Value as JsonValue};
use std::io::{self, BufRead, Read, Write};
use std::process::{Command, Stdio};

#[tokio::main]
async fn main() {
    // Persistent adapter: keep Codex MCP child alive across requests
    let mut bridge = match CodexBridge::spawn() {
        Ok(b) => b,
        Err(e) => {
            let _ = writeln!(io::stdout(), "{}", serde_json::to_string(&ToolResponse::err(format!("spawn codex bridge: {}", e))).unwrap());
            return;
        }
    };

    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let mut lines = stdin.lock().lines();
    while let Some(line_res) = lines.next() {
        let line = match line_res { Ok(l) => l, Err(e) => { let _ = writeln!(stdout, "{}", serde_json::to_string(&ToolResponse::err(format!("stdin error: {}", e))).unwrap()); break; } };
        let req = match serde_json::from_str::<ToolRequest>(&line) { Ok(r) => r, Err(e) => { let _ = writeln!(stdout, "{}", serde_json::to_string(&ToolResponse::err(format!("bad request: {}", e))).unwrap()); continue; } };
        let resp = handle(req, &mut bridge).await;
        let _ = writeln!(stdout, "{}", serde_json::to_string(&resp).unwrap());
        let _ = stdout.flush();
    }
}

async fn handle(req: ToolRequest, bridge: &mut CodexBridge) -> ToolResponse {
    let res = match req.tool.as_str() {
        // Start a new Codex run (returns session_id if observed)
        "new" => codex_run(bridge, None, &req.params),
        // Continue an existing session (requires session_id)
        "continue" => {
            // Accept both snake_case and camelCase from upstream; search recursively to be robust
            fn find_sid(v: &JsonValue) -> Option<String> {
                match v {
                    JsonValue::Object(map) => {
                        for (k, vv) in map {
                            let kl = k.to_ascii_lowercase();
                            if kl == "sessionid" || kl == "session_id" {
                                if let Some(s) = vv.as_str() { return Some(s.to_string()); }
                            }
                            if let Some(found) = find_sid(vv) { return Some(found); }
                        }
                        None
                    }
                    JsonValue::Array(arr) => {
                        for vv in arr { if let Some(found) = find_sid(vv) { return Some(found); } }
                        None
                    }
                    _ => None,
                }
            }
            let sid = find_sid(&req.params);
            if sid.is_none() {
                Err(anyhow!("missing session_id"))
            } else {
                codex_run(bridge, sid, &req.params)
            }
        }
        // Accept health pings from manager by returning a well-formed error
        _ => Err(anyhow!("unknown tool")),
    };
    match res { Ok(v) => ToolResponse::ok(v), Err(e) => ToolResponse::err(e.to_string()) }
}

// --- Persistent Codex JSON-RPC adapter over stdio ---

struct CodexBridge { rpc: JsonRpc }

impl CodexBridge {
    fn spawn() -> Result<Self> {
        let codex_bin = std::env::var("CODEX_BIN").unwrap_or_else(|_| "codex".into());
        let mut child = Command::new(codex_bin)
            .arg("mcp")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .context("spawn codex mcp")?;
        let stdin = child.stdin.take().ok_or_else(|| anyhow!("no stdin to codex"))?;
        let stdout = child.stdout.take().ok_or_else(|| anyhow!("no stdout from codex"))?;
        let mut rpc = JsonRpc::new(stdin, stdout);
        let _init: JsonValue = rpc.request(
            "initialize",
            json!({"protocolVersion":"2024-08-01","clientInfo":{"name":"foreman-codex-adapter","version": env!("CARGO_PKG_VERSION")},"capabilities":{}}),
        )?;
        Ok(Self { rpc })
    }
}

fn codex_run(bridge: &mut CodexBridge, session_id: Option<String>, params: &JsonValue) -> Result<JsonValue> {

    // tools/list
    let tools_list: JsonValue = bridge.rpc.request("tools/list", json!({}))?;
    let available: Vec<String> = tools_list
        .get("tools")
        .and_then(|v| v.as_array())
        .map(|a| a.iter().filter_map(|t| t.get("name").and_then(|n| n.as_str()).map(|s| s.to_string())).collect())
        .unwrap_or_default();

    // Pick tool for this action
    let tool_name = if session_id.is_some() {
        // continuation: prefer replay tool if present
        if available.iter().any(|n| n == "codex-replay") { "codex-replay" } else if available.iter().any(|n| n == "codex-reply") { "codex-reply" } else { "codex" }
    } else {
        // fresh run
        "codex"
    };

    // Build arguments with prompt/config and optional sessionId
    let prompt = params.get("prompt").and_then(|v| v.as_str()).unwrap_or("");
    let mut config = params.get("config").cloned().unwrap_or_else(|| json!({}));
    if let Some(repo) = params.get("repo").and_then(|v| v.as_str()) {
        // Pass repo path through config for Codex to consider
        if config.get("repo").is_none() {
            if let Some(cfg) = config.as_object_mut() { cfg.insert("repo".into(), json!(repo)); }
        }
    }
    if let Some(sid) = session_id.as_ref() {
        // Put sessionId in config and also at the top-level arguments for broader compatibility
        if let Some(cfg) = config.as_object_mut() { cfg.insert("sessionId".into(), json!(sid)); }
    }

    // Collect notifications while waiting for the call result
    let mut captured_session: Option<String> = None;
    let mut progress: Vec<JsonValue> = vec![];

    fn find_session_id(v: &JsonValue) -> Option<String> {
        match v {
            JsonValue::Object(map) => {
                for (k, vv) in map {
                    let kl = k.to_ascii_lowercase();
                    if kl == "sessionid" || kl == "session_id" {
                        if let Some(s) = vv.as_str() { return Some(s.to_string()); }
                    }
                    if let Some(found) = find_session_id(vv) { return Some(found); }
                }
                None
            }
            JsonValue::Array(arr) => {
                for vv in arr { if let Some(found) = find_session_id(vv) { return Some(found); } }
                None
            }
            _ => None,
        }
    }

    let mut hook = |method: &str, params: &JsonValue| {
        // Capture session id from any notification payload
        if captured_session.is_none() {
            if let Some(sid) = find_session_id(params) { captured_session = Some(sid); }
        }
        if method.starts_with("progress") || method == "status" {
            progress.push(params.clone());
        }
    };

    // tools/call
    // Build arguments allowing both styles: config.sessionId and top-level sessionId
    let mut arguments = json!({ "prompt": prompt, "config": config });
    if let Some(sid) = session_id.as_ref() {
        if let Some(map) = arguments.as_object_mut() { map.insert("sessionId".into(), json!(sid)); }
    }
    let call_res: JsonValue = bridge.rpc.request_with_hook(
        "tools/call",
        json!({ "name": tool_name, "arguments": arguments }),
        Some(&mut hook),
    )?;

    // If no notification carried the session id, try to extract from call result
    let effective_sid = captured_session.clone().or_else(|| find_session_id(&call_res)).or(session_id.clone());
    Ok(json!({
        "tool": tool_name,
        "session_id": effective_sid,
        "call_result": call_res,
        "progress": progress,
    }))
}

struct JsonRpc {
    id: i64,
    w: Box<dyn Write + Send>,
    r: Box<dyn Read + Send>,
}

impl JsonRpc {
    fn new(w: impl Write + Send + 'static, r: impl Read + Send + 'static) -> Self { Self { id: 1, w: Box::new(w), r: Box::new(r) } }

    fn request(&mut self, method: &str, params: JsonValue) -> Result<JsonValue> {
        self.request_with_hook(method, params, None)
    }

    fn request_with_hook(
        &mut self,
        method: &str,
        params: JsonValue,
        mut hook: Option<&mut dyn FnMut(&str, &JsonValue)>,
    ) -> Result<JsonValue> {
        let id = self.id;
        self.id += 1;
        let msg = json!({"jsonrpc":"2.0","id": id, "method": method, "params": params});
        self.write_message(&msg)?;
        // Read until matching id result
        loop {
            let v = self.read_message()?;
            if let Some(m) = v.get("method").and_then(|x| x.as_str()) {
                // notification
                if let Some(h) = hook.as_deref_mut() { let p = v.get("params").cloned().unwrap_or_else(|| json!({})); h(m, &p); }
                continue;
            }
            if v.get("id").and_then(|x| x.as_i64()) == Some(id) {
                if let Some(err) = v.get("error") { return Err(anyhow!(format!("rpc error: {}", err))); }
                let res = v.get("result").cloned().unwrap_or(JsonValue::Null);
                return Ok(res);
            }
            // unrelated response; ignore
        }
    }

    fn write_message(&mut self, v: &JsonValue) -> Result<()> {
        // Prefer newline-delimited JSON (Codex MCP server default). Fall back to Content-Length
        // only if needed (we don't auto-detect on write; newline JSON is more broadly tolerated).
        let mut body = serde_json::to_vec(v)?;
        body.push(b'\n');
        self.w.write_all(&body)?;
        self.w.flush()?;
        Ok(())
    }

    fn read_message(&mut self) -> Result<JsonValue> {
        // Be tolerant: ignore non-JSON log lines until we find a JSON message.
        // Also support Content-Length framed messages.
        let mut reader = io::BufReader::new(&mut self.r);
        let mut content_len: Option<usize> = None;
        loop {
            let mut line = String::new();
            let n = reader.read_line(&mut line)?;
            if n == 0 { return Err(anyhow!("eof")); }
            let trimmed = line.trim_end_matches(['\r', '\n']);
            if trimmed.is_empty() {
                // If we were in header mode and saw a blank line, read body by length.
                if let Some(len) = content_len.take() {
                    let mut buf = vec![0u8; len];
                    reader.read_exact(&mut buf)?;
                    let v: JsonValue = serde_json::from_slice(&buf)?;
                    return Ok(v);
                }
                continue;
            }
            let lower = trimmed.to_ascii_lowercase();
            if lower.starts_with("content-length:") {
                if let Some(rest) = trimmed.splitn(2, ':').nth(1) {
                    if let Ok(len) = rest.trim().parse::<usize>() { content_len = Some(len); }
                }
                continue;
            }
            // Try parse as JSON; otherwise ignore as noise (logs)
            if let Ok(v) = serde_json::from_str::<JsonValue>(trimmed) { return Ok(v); }
        }
    }
}
