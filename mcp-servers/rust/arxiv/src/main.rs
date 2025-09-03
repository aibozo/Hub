use foreman_mcp::{ToolRequest, ToolResponse};
use mcp_arxiv as arxiv;
use std::io::{self, BufRead, Write};

#[tokio::main]
async fn main() {
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let mut lines = stdin.lock().lines();
    while let Some(Ok(line)) = lines.next() {
        if line.trim().is_empty() { continue; }
        let req: Result<ToolRequest, _> = serde_json::from_str(&line);
        let resp = match req {
            Ok(r) => handle(r).await,
            Err(e) => ToolResponse::err(format!("bad request: {}", e)),
        };
        let _ = writeln!(stdout, "{}", serde_json::to_string(&resp).unwrap());
        let _ = stdout.flush();
    }
}

async fn handle(req: ToolRequest) -> ToolResponse {
    let res = match req.tool.as_str() {
        // Match existing manifest tool names for drop-in replacement
        "search" => arxiv::search(&req.params).await,
        "top" => arxiv::top(&req.params).await,
        "summarize" => arxiv::summarize(&req.params).await,
        "fetch_pdf" => arxiv::fetch_pdf(&req.params).await,
        _ => Err(anyhow::anyhow!("unknown tool")),
    };
    match res { Ok(v) => ToolResponse::ok(v), Err(e) => ToolResponse::err(e.to_string()) }
}
