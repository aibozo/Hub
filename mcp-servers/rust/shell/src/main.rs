use foreman_mcp::{ToolRequest, ToolResponse};
use mcp_shell as shell;
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
        // Back-compat aliases with server-prefixed names
        "shell_list_dir" | "list_dir" => {
            let path = req.params.get("path").and_then(|v| v.as_str()).unwrap_or(".");
            shell::list_dir(path).await
        }
        "shell_read_file" | "read_file" => {
            let path = req.params.get("path").and_then(|v| v.as_str()).unwrap_or("");
            shell::read_file(path).await
        }
        "shell_which" | "which" => {
            let cmd = req.params.get("cmd").and_then(|v| v.as_str()).unwrap_or("");
            shell::which_cmd(cmd).await
        }
        "shell_exec" | "exec" => {
            let cmd = req.params.get("cmd").and_then(|v| v.as_str()).unwrap_or("");
            let args: Vec<String> = req
                .params
                .get("args")
                .and_then(|v| v.as_array())
                .map(|a| a.iter().filter_map(|x| x.as_str().map(|s| s.to_string())).collect())
                .unwrap_or_default();
            let wait = req.params.get("wait").and_then(|v| v.as_bool()).unwrap_or(true);
            shell::exec(cmd, &args, wait).await
        }
        _ => Err(anyhow::anyhow!("unknown tool")),
    };
    match res {
        Ok(v) => ToolResponse::ok(v),
        Err(e) => ToolResponse::err(e.to_string()),
    }
}
