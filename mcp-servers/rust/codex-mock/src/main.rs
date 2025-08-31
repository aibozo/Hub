use anyhow::{anyhow, Result};
use serde_json::{json, Value as JsonValue};
use std::io::{self, BufRead, Write};

fn main() -> Result<()> {
    // Accept optional subcommand (expect "mcp")
    let mut args = std::env::args().skip(1);
    if let Some(sub) = args.next() {
        if sub != "mcp" { eprintln!("codex-mock: ignoring subcommand {sub}"); }
    }
    run_mcp()
}

fn run_mcp() -> Result<()> {
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let mut reader = io::BufRead::lines(io::BufReader::new(stdin));
    // Simple JSON-RPC over newline-delimited JSON. No Content-Length framing.
    while let Some(Ok(line)) = reader.next() {
        let v: JsonValue = match serde_json::from_str(&line) { Ok(v) => v, Err(_) => continue };
        if let Some(id) = v.get("id").and_then(|x| x.as_i64()) {
            let method = v.get("method").and_then(|x| x.as_str()).unwrap_or("");
            match method {
                "initialize" => {
                    let resp = json!({"jsonrpc":"2.0","id": id, "result": {"serverInfo": {"name":"codex-mock","version":"0.1.0"}}});
                    writeln!(stdout, "{}", serde_json::to_string(&resp)?)?;
                    stdout.flush()?;
                }
                "tools/list" => {
                    let resp = json!({"jsonrpc":"2.0","id": id, "result": {"tools": [
                        {"name":"codex","description":"Start a new Codex run"},
                        {"name":"codex-replay","description":"Continue a Codex session"}
                    ]}});
                    writeln!(stdout, "{}", serde_json::to_string(&resp)?)?;
                    stdout.flush()?;
                }
                "tools/call" => {
                    // Emit a notification announcing session id
                    let notif = json!({"jsonrpc":"2.0","method":"session_configured","params": {"msg": {"session_id": "mock-123"}}});
                    writeln!(stdout, "{}", serde_json::to_string(&notif)?)?;
                    // Echo arguments back in result
                    let args = v.get("params").and_then(|p| p.get("arguments")).cloned().unwrap_or_else(|| json!({}));
                    let resp = json!({"jsonrpc":"2.0","id": id, "result": {"ok": true, "echo": args}});
                    writeln!(stdout, "{}", serde_json::to_string(&resp)?)?;
                    stdout.flush()?;
                    break; // single-call mock
                }
                _ => {
                    let resp = json!({"jsonrpc":"2.0","id": id, "error": {"code": -32601, "message": "method not found"}});
                    writeln!(stdout, "{}", serde_json::to_string(&resp)?)?;
                    stdout.flush()?;
                }
            }
        } else if v.get("method").and_then(|x| x.as_str()).is_some() {
            // Ignore notifications from client
            continue;
        } else {
            return Err(anyhow!("invalid JSON-RPC message"));
        }
    }
    Ok(())
}

