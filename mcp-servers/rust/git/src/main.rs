use foreman_mcp::{ToolRequest, ToolResponse};
use mcp_git as git_svr;
use std::io::{self, BufRead, Write};

#[tokio::main]
async fn main() {
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let mut lines = stdin.lock().lines();
    if let Some(Ok(line)) = lines.next() {
        let req: Result<ToolRequest, _> = serde_json::from_str(&line);
        let resp = match req {
            Ok(r) => handle(r).await,
            Err(e) => ToolResponse::err(format!("bad request: {}", e)),
        };
        let _ = writeln!(stdout, "{}", serde_json::to_string(&resp).unwrap());
    }
}

async fn handle(req: ToolRequest) -> ToolResponse {
    let res = match req.tool.as_str() {
        "status" => {
            let path = req.params.get("path").and_then(|v| v.as_str()).unwrap_or(".");
            git_svr::status(path).await
        }
        _ => Err(anyhow::anyhow!("unknown tool")),
    };
    match res {
        Ok(v) => ToolResponse::ok(v),
        Err(e) => ToolResponse::err(e.to_string()),
    }
}

