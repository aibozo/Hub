use serde::{Deserialize};
use serde_json::{json, Value as JsonValue};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
use tokio::process::Command;
use tokio::time::{timeout, Duration};
use once_cell::sync::OnceCell;
use std::sync::Arc;
use tokio::sync::Mutex as AsyncMutex;

#[derive(Clone, Debug)]
pub struct ToolManifest {
    pub server: String,
    pub tools: Vec<String>,
    pub transport: Option<String>,
    pub bin: Option<String>,
    pub autostart: bool,
}

#[derive(Clone, Default)]
pub struct ToolsManager {
    manifests: HashMap<String, ToolManifest>,
    clients: Arc<AsyncMutex<HashMap<String, Arc<AsyncMutex<StdioClient>>>>>,
}

impl ToolsManager {
    pub fn load_from_dir(dir: &Path) -> Self {
        let mut m = ToolsManager { manifests: HashMap::new(), clients: Arc::new(AsyncMutex::new(HashMap::new())) };
        if let Ok(rd) = fs::read_dir(dir) {
            for ent in rd.flatten() {
                if let Some(name) = ent.file_name().to_str() {
                    if name.ends_with(".json") {
                        if let Ok(text) = fs::read_to_string(ent.path()) {
                            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) {
                                let server = v.get("server").and_then(|s| s.as_str()).unwrap_or("").to_string();
                                let tools = v.get("tools").and_then(|t| t.as_array()).map(|a| a.iter().filter_map(|x| x.as_str().map(|s| s.to_string())).collect()).unwrap_or_default();
                                let transport = v.get("transport").and_then(|s| s.as_str()).map(|s| s.to_string());
                                let bin = v.get("bin").and_then(|s| s.as_str()).map(|s| s.to_string());
                                let autostart = v.get("autostart").and_then(|b| b.as_bool()).unwrap_or(false);
                                if !server.is_empty() {
                                    m.manifests.insert(server.clone(), ToolManifest { server, tools, transport, bin, autostart });
                                }
                            }
                        }
                    }
                }
            }
        }
        m
    }

    pub fn servers(&self) -> Vec<String> { self.manifests.keys().cloned().collect() }

    pub fn list(&self) -> Vec<(String, Vec<String>)> {
        let mut v: Vec<(String, Vec<String>)> = self
            .manifests
            .iter()
            .map(|(k, m)| (k.clone(), m.tools.clone()))
            .collect();
        v.sort_by(|a, b| a.0.cmp(&b.0));
        v
    }

    pub async fn invoke(&self, server: &str, tool: &str, params: JsonValue) -> anyhow::Result<JsonValue> {
        // Back-compat: allow server-prefixed tool names like "shell_exec" via aliasing
        let tool_aliased: String = if server == "shell" {
            match tool {
                "shell_exec" => "exec".into(),
                "shell_list_dir" => "list_dir".into(),
                "shell_read_file" => "read_file".into(),
                "shell_which" => "which".into(),
                _ => tool.to_string(),
            }
        } else { tool.to_string() };
        let tool = tool_aliased.as_str();
        // Special-case shell.exec: enforce a strict whitelist before invoking MCP
        if server == "shell" && tool == "exec" {
            validate_shell_exec(&params)?;
        }
        // Prefer external MCP if configured
        let mut stdio_err: Option<anyhow::Error> = None;
        if let Some(man) = self.manifests.get(server) {
            if man.transport.as_deref() == Some("stdio") {
                if let Some(bin) = man.bin.as_ref() {
                    // Persistent stdio client per server
                    let client = self.get_or_spawn_client(server, bin).await;
                    match client {
                        Ok(cli) => {
                            let req = foreman_mcp::ToolRequest { tool: tool.to_string(), params: params.clone() };
                            let mut guard = cli.lock().await;
                            match guard.send(req).await {
                                Ok(resp) => { return if resp.ok { Ok(resp.result) } else { anyhow::bail!(resp.error.unwrap_or_else(|| "unknown error".into())) }; }
                                Err(e) => {
                                    // Try one respawn and retry
                                    let mut map = self.clients.lock().await;
                                    match StdioClient::spawn(bin) {
                                        Ok(c) => {
                                            let new_cli = Arc::new(AsyncMutex::new(c));
                                            map.insert(server.to_string(), new_cli.clone());
                                            drop(map);
                                            drop(guard);
                                            let req2 = foreman_mcp::ToolRequest { tool: tool.to_string(), params: params.clone() };
                                            let mut g2 = new_cli.lock().await;
                                            match g2.send(req2).await {
                                                Ok(resp2) => { return if resp2.ok { Ok(resp2.result) } else { anyhow::bail!(resp2.error.unwrap_or_else(|| "unknown error".into())) }; }
                                                Err(e2) => { stdio_err = Some(e2); }
                                            }
                                        }
                                        Err(spawn_err) => {
                                            stdio_err = Some(anyhow::anyhow!(format!("spawn failed: {} (prior err: {})", spawn_err, e)));
                                            map.remove(server);
                                        }
                                    }
                                }
                            }
                        }
                        Err(e) => { stdio_err = Some(e); }
                    }
                }
            }
        }
        // Fall back to in-core stubs
        match server {
            "shell" => invoke_shell(tool, params).await,
            "fs" => invoke_fs(tool, params).await,
            "proc" => invoke_proc(tool, params).await,
            "git" => invoke_git(tool, params).await,
            "arxiv" => invoke_arxiv(tool, params).await,
            "news" => invoke_news(tool, params).await,
            "installer" => invoke_installer(tool, params).await,
            "steam" => invoke_steam(tool, params, self).await,
            "project" => invoke_project(tool, params).await,
            _ => {
                if let Some(e) = stdio_err { Err(e) } else { Err(anyhow::anyhow!("unknown server")) }
            }
        }
    }
}

async fn invoke_mcp_stdio(bin: &str, tool: &str, params: JsonValue) -> anyhow::Result<JsonValue> {
    let req = foreman_mcp::ToolRequest { tool: tool.to_string(), params };
    // Allow complex command strings like "python -m server"
    let mut child = if bin.contains(' ') {
        let mut cmd = Command::new("sh");
        cmd.arg("-lc").arg(bin).stdin(std::process::Stdio::piped()).stdout(std::process::Stdio::piped());
        cmd.spawn()?
    } else {
        let mut cmd = Command::new(bin);
        cmd.stdin(std::process::Stdio::piped()).stdout(std::process::Stdio::piped());
        cmd.spawn()?
    };

    // Write request line
    if let Some(mut stdin) = child.stdin.take() {
        let line = serde_json::to_string(&req)? + "\n";
        stdin.write_all(line.as_bytes()).await?;
    } else {
        anyhow::bail!("failed to open stdin for {}", bin);
    }

    // Read single-line response with generous timeout (Codex can take minutes)
    let stdout = child.stdout.take().ok_or_else(|| anyhow::anyhow!("no stdout"))?;
    let mut reader = tokio::io::BufReader::new(stdout).lines();
    // Default to 5 minutes (300s) to accommodate long Codex runs.
    let line = timeout(Duration::from_secs(300), reader.next_line()).await??.ok_or_else(|| anyhow::anyhow!("no response"))?;
    // Ensure process ends
    let _ = timeout(Duration::from_secs(2), child.wait()).await;

    let resp: foreman_mcp::ToolResponse = serde_json::from_str(&line).map_err(|e| anyhow::anyhow!("bad MCP response: {}", e))?;
    if resp.ok { Ok(resp.result) } else { anyhow::bail!(resp.error.unwrap_or_else(|| "unknown error".into())) }
}

impl ToolsManager {
    pub async fn autostart(&self) {
        for (name, man) in self.manifests.iter() {
            if man.autostart && man.transport.as_deref() == Some("stdio") {
                if let Some(bin) = &man.bin {
                    let name = name.clone();
                    let bin = bin.clone();
                    let this = self.clone();
                    tokio::spawn(async move {
                        let _ = this.get_or_spawn_client(&name, &bin).await;
                    });
                }
            }
        }
    }

    pub async fn statuses(&self) -> Vec<(String, String)> {
        let mut out: Vec<(String, String)> = vec![];
        for (name, man) in self.manifests.iter() {
            let status = if man.transport.as_deref() == Some("stdio") {
                if let Some(bin) = &man.bin {
                    match ping_once(bin).await {
                        Ok(()) => "Connected".to_string(),
                        Err(e) => format!("Error: {}", truncate_err(&e.to_string())),
                    }
                } else {
                    "NotConfigured".to_string()
                }
            } else {
                "InCore".to_string()
            };
            out.push((name.clone(), status));
        }
        out.sort_by(|a, b| a.0.cmp(&b.0));
        out
    }
}

// Build tool schemas for Realtime session.update (flat function schema form)
pub fn realtime_tool_schemas(tm: &ToolsManager) -> Vec<serde_json::Value> {
    let mut out = vec![];
    for (server, tools) in tm.list().into_iter() {
        for t in tools {
            // Tool names must match ^[a-zA-Z0-9_-]+$ in Realtime; use underscores
            let name = format!("{}_{}", server, t).replace('-', "_");
            let (desc, params): (String, serde_json::Value) = if server == "shell" && t == "exec" {
                (
                    "Execute a desktop command with strict policy. Usage: {\"cmd\":\"mgba-qt\",\"args\":[\"/home/kil/games/roms/<console>/<file>\"]} (GB/GBA). For Nintendo DS: {\"cmd\":\"/home/kil/games/emulators/melonDS-x86_64.AppImage\",\"args\":[]}. Optional: {\"wait\": false} to spawn and return a PID.".to_string(),
                    serde_json::json!({
                        "type": "object",
                        "properties": {
                            "cmd": {"type": "string", "description": "Program to run. 'mgba-qt' or absolute DS emulator path."},
                            "args": {"type": "array", "items": {"type": "string"}, "description": "Arguments; for mgba-qt, one ROM path under /home/kil/games/roms"},
                            "wait": {"type": "boolean", "description": "If false, spawn and return pid; defaults to true."}
                        },
                        "required": ["cmd"],
                        "additionalProperties": false
                    })
                )
            } else if server == "shell" && t == "which" {
                (
                    "Resolve a command name to a full path (PATH lookup).".to_string(),
                    serde_json::json!({
                        "type": "object",
                        "properties": {"cmd": {"type": "string", "description": "Command name, e.g., 'mgba-qt'"}},
                        "required": ["cmd"],
                        "additionalProperties": false
                    })
                )
            } else if server == "shell" && (t == "list_dir" || t == "read_file") {
                let d = if t == "list_dir" { "List entries in a directory." } else { "Read a UTF-8 text file." };
                (
                    d.to_string(),
                    serde_json::json!({
                        "type": "object",
                        "properties": {"path": {"type": "string", "description": "Filesystem path"}},
                        "required": ["path"],
                        "additionalProperties": false
                    })
                )
            } else if server == "steam" && t == "installed" {
                (
                    "List installed Steam games by reading local Steam app manifests (~/.local/share/Steam/steamapps).".to_string(),
                    serde_json::json!({
                        "type": "object",
                        "properties": {
                            "root": {"type": "string", "description": "Optional override for steamapps directory"}
                        },
                        "additionalProperties": false
                    })
                )
            } else if server == "steam" && t == "launch" {
                (
                    "Launch a Steam game (steam -applaunch APPID).".to_string(),
                    serde_json::json!({
                        "type": "object",
                        "properties": {"appid": {"type": "string", "description": "Steam AppID (digits)"}},
                        "required": ["appid"],
                        "additionalProperties": false
                    })
                )
            } else {
                (
                    format!("Call {}.{} via Foreman MCP", server, t),
                    serde_json::json!({"type": "object", "properties": {}, "additionalProperties": true})
                )
            };
            out.push(serde_json::json!({
                "type": "function",
                "name": name,
                "description": desc,
                "parameters": params
            }));
        }
    }
    // Add synthetic end_call tool
    out.push(serde_json::json!({
        "type": "function",
        "name": "end_call",
        "description": "End the voice session and return to text chat.",
        "parameters": {"type": "object", "properties": {}, "additionalProperties": false}
    }));
    out
}

async fn ping_server(bin: String, name: String) {
    let _ = ping_once(&bin).await.map_err(|e| tracing::warn!(server=%name, error=%e, "mcp autostart ping failed"));
}

async fn ping_once(bin: &str) -> anyhow::Result<()> {
    // Send an unknown tool; treat any well-formed response as success
    let req = foreman_mcp::ToolRequest { tool: "health".into(), params: json!({}) };
    let mut child = if bin.contains(' ') {
        let mut cmd = Command::new("sh");
        cmd.arg("-lc").arg(bin).stdin(std::process::Stdio::piped()).stdout(std::process::Stdio::piped());
        cmd.spawn()?
    } else {
        let mut cmd = Command::new(bin);
        cmd.stdin(std::process::Stdio::piped()).stdout(std::process::Stdio::piped());
        cmd.spawn()?
    };
    if let Some(mut stdin) = child.stdin.take() {
        let line = serde_json::to_string(&req)? + "\n";
        stdin.write_all(line.as_bytes()).await?;
    }
    let stdout = child.stdout.take().ok_or_else(|| anyhow::anyhow!("no stdout"))?;
    let mut reader = tokio::io::BufReader::new(stdout).lines();
    let _ = timeout(Duration::from_secs(3), reader.next_line()).await??.ok_or_else(|| anyhow::anyhow!("no response"))?;
    let _ = timeout(Duration::from_secs(2), child.wait()).await;
    Ok(())
}

struct StdioClient {
    bin: String,
    child: tokio::process::Child,
    stdin: tokio::process::ChildStdin,
    lines: tokio::io::Lines<tokio::io::BufReader<tokio::process::ChildStdout>>,
}

impl StdioClient {
    fn spawn(bin: &str) -> anyhow::Result<Self> {
        let mut child = if bin.contains(' ') {
            let mut cmd = Command::new("sh");
            cmd.arg("-lc").arg(bin).stdin(std::process::Stdio::piped()).stdout(std::process::Stdio::piped());
            cmd.spawn()?
        } else {
            let mut cmd = Command::new(bin);
            cmd.stdin(std::process::Stdio::piped()).stdout(std::process::Stdio::piped());
            cmd.spawn()?
        };
        let stdin = child.stdin.take().ok_or_else(|| anyhow::anyhow!("no stdin for client"))?;
        let stdout = child.stdout.take().ok_or_else(|| anyhow::anyhow!("no stdout for client"))?;
        let reader = tokio::io::BufReader::new(stdout);
        Ok(Self { bin: bin.to_string(), child, stdin, lines: tokio::io::BufReader::lines(reader) })
    }

    async fn send(&mut self, req: foreman_mcp::ToolRequest) -> anyhow::Result<foreman_mcp::ToolResponse> {
        let line = serde_json::to_string(&req)? + "\n";
        self.stdin.write_all(line.as_bytes()).await?;
        let l = timeout(Duration::from_secs(300), self.lines.next_line()).await??.ok_or_else(|| anyhow::anyhow!("no response"))?;
        // Some servers may emit logs; attempt to resync if we parse non-JSON
        match serde_json::from_str::<foreman_mcp::ToolResponse>(&l) {
            Ok(r) => Ok(r),
            Err(_) => {
                // Keep reading until JSON
                loop {
                    let ln = timeout(Duration::from_secs(300), self.lines.next_line()).await??.ok_or_else(|| anyhow::anyhow!("no response"))?;
                    if let Ok(r) = serde_json::from_str::<foreman_mcp::ToolResponse>(&ln) { return Ok(r); }
                }
            }
        }
    }
}

impl ToolsManager {
    async fn get_or_spawn_client(&self, server: &str, bin: &str) -> anyhow::Result<Arc<AsyncMutex<StdioClient>>> {
        let mut map = self.clients.lock().await;
        if let Some(c) = map.get(server).cloned() { return Ok(c); }
        let cli = Arc::new(AsyncMutex::new(StdioClient::spawn(bin)?));
        map.insert(server.to_string(), cli.clone());
        Ok(cli)
    }
}

fn truncate_err(s: &str) -> String { if s.len() > 60 { format!("{}…", &s[..60]) } else { s.to_string() } }

#[derive(Debug, Clone, Deserialize)]
struct ShellArgsRule {
    #[serde(default)]
    count: Option<usize>,
    #[serde(default)]
    min_count: Option<usize>,
    #[serde(default)]
    max_count: Option<usize>,
    #[serde(default)]
    path_prefixes: Option<Vec<String>>, // all args must start with one of these prefixes (if provided)
    #[serde(default)]
    starts_with_tokens: Option<Vec<String>>, // e.g., ["-applaunch"]
    #[serde(default)]
    digits_at: Option<usize>, // index that must be numeric (e.g., appid)
}

#[derive(Debug, Clone, Deserialize)]
struct ShellRule {
    #[serde(rename = "match")]
    r#match: String, // basename or absolute path match
    #[serde(default)]
    args: Option<ShellArgsRule>,
}

#[derive(Debug, Clone, Deserialize, Default)]
struct ShellAllowFile { #[serde(default)] shell_allowlist: Vec<ShellRule> }

static SHELL_RULES: OnceCell<Vec<ShellRule>> = OnceCell::new();

fn load_shell_rules() -> Vec<ShellRule> {
    if let Some(r) = SHELL_RULES.get() { return r.clone(); }
    let dir = std::path::Path::new("config/policy.d");
    let mut rules: Vec<ShellRule> = vec![];
    if let Ok(rd) = std::fs::read_dir(dir) {
        let mut ents: Vec<_> = rd.flatten().collect();
        ents.sort_by_key(|e| e.file_name());
        for e in ents {
            if e.path().extension().and_then(|s| s.to_str()) == Some("yaml") {
                if let Ok(text) = std::fs::read_to_string(e.path()) {
                    if let Ok(file) = serde_yaml::from_str::<ShellAllowFile>(&text) {
                        rules.extend(file.shell_allowlist);
                    }
                }
            }
        }
    }
    // Built-in defaults if none provided
    if rules.is_empty() {
        rules.push(ShellRule { r#match: "mgba-qt".into(), args: Some(ShellArgsRule { count: Some(1), min_count: None, max_count: None, path_prefixes: Some(vec!["/home/kil/games/roms".into()]), starts_with_tokens: None, digits_at: None }) });
        rules.push(ShellRule { r#match: "/home/kil/games/emulators/melonDS-x86_64.AppImage".into(), args: Some(ShellArgsRule { count: Some(0), min_count: None, max_count: None, path_prefixes: None, starts_with_tokens: None, digits_at: None }) });
        rules.push(ShellRule { r#match: "melonDS-x86_64.AppImage".into(), args: Some(ShellArgsRule { count: Some(0), min_count: None, max_count: None, path_prefixes: None, starts_with_tokens: None, digits_at: None }) });
    }
    let _ = SHELL_RULES.set(rules.clone());
    rules
}

fn validate_shell_exec(params: &JsonValue) -> anyhow::Result<()> {
    fn trim_unquote(s: &str) -> String {
        let t = s.trim();
        if (t.starts_with('"') && t.ends_with('"')) || (t.starts_with('\'') && t.ends_with('\'')) {
            t[1..t.len().saturating_sub(1)].to_string()
        } else { t.to_string() }
    }
    fn basename(s: &str) -> &str { std::path::Path::new(s).file_name().and_then(|x| x.to_str()).unwrap_or(s) }

    let raw_cmd = params.get("cmd").and_then(|v| v.as_str()).unwrap_or("");
    let cmd = trim_unquote(raw_cmd);
    let cmd_name = basename(&cmd);
    let mut args: Vec<String> = params
        .get("args")
        .and_then(|v| v.as_array())
        .map(|a| a.iter().filter_map(|x| x.as_str().map(|s| trim_unquote(s))).collect())
        .unwrap_or_default();

    // Load rules
    let rules = load_shell_rules();
    // Try to match any rule
    for r in rules.iter() {
        if r.r#match == cmd || r.r#match == cmd_name {
            // Args policy
            if let Some(a) = &r.args {
                if let Some(c) = a.count { if args.len() != c { continue; } }
                if let Some(minc) = a.min_count { if args.len() < minc { continue; } }
                if let Some(maxc) = a.max_count { if args.len() > maxc { continue; } }
                if let Some(prefixes) = &a.path_prefixes {
                    // require all args to start with any allowed prefix
                    if !args.iter().all(|s| prefixes.iter().any(|p| s.starts_with(p))) { continue; }
                }
                if let Some(tokens) = &a.starts_with_tokens {
                    if args.len() < tokens.len() { continue; }
                    let mut ok = true;
                    for (i, t) in tokens.iter().enumerate() {
                        if &args[i] != t { ok = false; break; }
                    }
                    if !ok { continue; }
                }
                if let Some(ix) = a.digits_at {
                    if args.get(ix).map(|s| s.chars().all(|c| c.is_ascii_digit())).unwrap_or(false) == false { continue; }
                }
            }
            return Ok(());
        }
    }
    anyhow::bail!(format!("command not whitelisted: {} (see config/policy.d/*shell* for allowlist)", cmd))
}

async fn invoke_shell(tool: &str, params: JsonValue) -> anyhow::Result<JsonValue> {
    match tool {
        "list_dir" => {
            let path = params.get("path").and_then(|v| v.as_str()).unwrap_or(".");
            let mut entries: Vec<String> = vec![];
            let mut dir = tokio::fs::read_dir(path).await?;
            while let Ok(Some(ent)) = dir.next_entry().await {
                if let Ok(name) = ent.file_name().into_string() { entries.push(name); }
            }
            entries.sort();
            Ok(json!({ "entries": entries }))
        }
        "read_file" => {
            let path = params.get("path").and_then(|v| v.as_str()).unwrap_or("");
            let data = tokio::fs::read_to_string(path).await?;
            Ok(json!({ "content": data }))
        }
        "which" => {
            let cmd = params.get("cmd").and_then(|v| v.as_str()).unwrap_or("");
            let output = tokio::process::Command::new("which").arg(cmd).output().await?;
            let path = if output.status.success() {
                Some(String::from_utf8_lossy(&output.stdout).lines().next().unwrap_or("").trim().to_string())
            } else { None };
            Ok(json!({ "path": path }))
        }
        _ => Err(anyhow::anyhow!("unknown tool")),
    }
}

async fn invoke_fs(tool: &str, params: JsonValue) -> anyhow::Result<JsonValue> {
    match tool {
        "stat" => {
            let path = params.get("path").and_then(|v| v.as_str()).unwrap_or(".");
            let meta = tokio::fs::metadata(path).await?;
            let is_dir = meta.is_dir();
            let is_file = meta.is_file();
            let size = if is_file { Some(meta.len()) } else { None };
            Ok(json!({ "is_dir": is_dir, "is_file": is_file, "size": size }))
        }
        "list" => {
            let path = params.get("path").and_then(|v| v.as_str()).unwrap_or(".");
            let mut entries: Vec<String> = vec![];
            let mut dir = tokio::fs::read_dir(path).await?;
            while let Ok(Some(ent)) = dir.next_entry().await {
                if let Ok(name) = ent.file_name().into_string() { entries.push(name); }
            }
            entries.sort();
            Ok(json!({ "entries": entries }))
        }
        "read" => {
            let path = params.get("path").and_then(|v| v.as_str()).unwrap_or("");
            let content = tokio::fs::read_to_string(path).await?;
            Ok(json!({ "content": content }))
        }
        _ => Err(anyhow::anyhow!("unknown tool")),
    }
}

async fn invoke_proc(tool: &str, _params: JsonValue) -> anyhow::Result<JsonValue> {
    match tool {
        "list" => {
            let mut pids: Vec<i32> = vec![];
            let mut dir = tokio::fs::read_dir("/proc").await?;
            while let Ok(Some(ent)) = dir.next_entry().await {
                if let Ok(name) = ent.file_name().into_string() {
                    if let Ok(pid) = name.parse::<i32>() { pids.push(pid); }
                }
            }
            pids.sort();
            Ok(json!({ "pids": pids }))
        }
        _ => Err(anyhow::anyhow!("unknown tool")),
    }
}

async fn invoke_git(tool: &str, params: JsonValue) -> anyhow::Result<JsonValue> {
    match tool {
        "status" => {
            let path = params.get("path").and_then(|v| v.as_str()).unwrap_or(".");
            let git_dir = format!("{}/.git", path.trim_end_matches('/'));
            let is_repo = tokio::fs::metadata(&git_dir).await.is_ok();
            if !is_repo { return Ok(json!({ "repo": false })); }
            let out = tokio::process::Command::new("git").arg("-C").arg(path).arg("status").arg("--porcelain").output().await?;
            let ok = out.status.success();
            let changed = String::from_utf8_lossy(&out.stdout).lines().count();
            Ok(json!({ "repo": true, "ok": ok, "changed": changed }))
        }
        _ => Err(anyhow::anyhow!("unknown tool")),
    }
}

// ---- Steam helpers (installed list, launch wrapper) ----
async fn invoke_steam(tool: &str, params: JsonValue, tm: &ToolsManager) -> anyhow::Result<JsonValue> {
    match tool {
        "installed" => {
            let mut out: Vec<(String, String)> = vec![]; // (appid, name)
            let home = std::env::var("HOME").unwrap_or_else(|_| String::from(""));
            let candidates = vec![
                format!("{}/.local/share/Steam/steamapps", home),
                format!("{}/.steam/steam/steamapps", home),
                format!("{}/.var/app/com.valvesoftware.Steam/data/Steam/steamapps", home),
            ];
            for dir in candidates {
                if let Ok(mut rd) = tokio::fs::read_dir(&dir).await {
                    while let Ok(Some(ent)) = rd.next_entry().await {
                        if let Ok(name) = ent.file_name().into_string() {
                            if name.starts_with("appmanifest_") && name.ends_with(".acf") {
                                if let Ok(content) = tokio::fs::read_to_string(ent.path()).await {
                                    if let Some((id, title)) = parse_appmanifest(&content) {
                                        out.push((id, title));
                                    }
                                }
                            }
                        }
                    }
                }
            }
            out.sort_by(|a,b| a.1.cmp(&b.1));
            Ok(json!({"games": out.into_iter().map(|(id,name)| json!({"appid": id, "name": name})).collect::<Vec<_>>()}))
        }
        "launch" => {
            // Proxy to shell.exec with validated args
            let appid_str: Option<String> = if let Some(s) = params.get("appid").and_then(|v| v.as_str()) { Some(s.to_string()) } else if let Some(n) = params.get("appid").and_then(|v| v.as_u64()) { Some(n.to_string()) } else { None };
            let appid = appid_str.ok_or_else(|| anyhow::anyhow!("appid required"))?;
            let shell_params = json!({"cmd": "steam", "args": ["-applaunch", appid]});
            // Preflight against allowlist, then invoke shell directly to avoid async recursion issues
            validate_shell_exec(&shell_params)?;
            invoke_shell("exec", shell_params).await
        }
        _ => Err(anyhow::anyhow!("unknown tool")),
    }
}

fn parse_appmanifest(text: &str) -> Option<(String, String)> {
    // Extremely lenient parse: find first appid and name tokens
    let mut appid: Option<String> = None;
    let mut name: Option<String> = None;
    for line in text.lines() {
        let l = line.trim();
        if appid.is_none() {
            if let Some(i) = l.find("\"appid\"") {
                // expect: "appid"  "12345"
                if let Some(q1) = l[i..].find('"') {
                    if let Some(q2) = l[i+q1+1..].find('"') {
                        let rest = &l[i+q1+1+q2+1..];
                        if let Some(s) = rest.split('"').nth(1) { appid = Some(s.to_string()); }
                    }
                }
            }
        }
        if name.is_none() {
            if let Some(i) = l.find("\"name\"") {
                if let Some(q1) = l[i..].find('"') {
                    if let Some(q2) = l[i+q1+1..].find('"') {
                        let rest = &l[i+q1+1+q2+1..];
                        if let Some(s) = rest.split('"').nth(1) { name = Some(s.to_string()); }
                    }
                }
            }
        }
        if appid.is_some() && name.is_some() { break; }
    }
    match (appid, name) { (Some(i), Some(n)) => Some((i,n)), _ => None }
}

async fn invoke_arxiv(tool: &str, params: JsonValue) -> anyhow::Result<JsonValue> {
    match tool {
        "search" => {
            let q = params.get("query").and_then(|v| v.as_str()).unwrap_or("");
            let results: Vec<serde_json::Value> = (1..=5)
                .map(|i| serde_json::json!({
                    "id": format!("arXiv:{}{:04}", chrono::Local::now().format("%Y.%m"), i),
                    "title": format!("{} — result {}", q, i),
                    "authors": ["Doe, J.", "Smith, A."],
                    "date": chrono::Local::now().format("%Y-%m-%d").to_string()
                }))
                .collect();
            Ok(serde_json::json!({"results": results}))
        }
        "top" => {
            let month = params.get("month").and_then(|v| v.as_str()).unwrap_or(&chrono::Local::now().format("%Y-%m").to_string()).to_string();
            let n = params.get("n").and_then(|v| v.as_u64()).unwrap_or(5) as usize;
            let items: Vec<serde_json::Value> = (1..=n)
                .map(|i| serde_json::json!({
                    "id": format!("arXiv:{}.{:04}", month.replace("-", ""), i),
                    "title": format!("Top {} paper #{}", month, i)
                }))
                .collect();
            Ok(serde_json::json!({"month": month, "items": items}))
        }
        "summarize" => {
            let id = params.get("id").and_then(|v| v.as_str()).unwrap_or("unknown");
            Ok(serde_json::json!({"summary": format!("Summary for {} (stub)", id)}))
        }
        "fetch_pdf" => {
            let id = params.get("id").and_then(|v| v.as_str()).unwrap_or("arXiv:unknown");
            let fname = format!("{}.pdf", id.replace(':', "_").replace('/', "_"));
            let cache_dir = std::path::PathBuf::from("storage/arxiv_cache");
            tokio::fs::create_dir_all(&cache_dir).await.ok();
            let path = cache_dir.join(fname);
            let content = b"%PDF-1.4\n% Stub PDF content for testing\n";
            let _ = tokio::fs::write(&path, content).await;
            Ok(serde_json::json!({"path": path.to_string_lossy()}))
        }
        _ => Err(anyhow::anyhow!("unknown tool")),
    }
}

async fn invoke_news(tool: &str, params: JsonValue) -> anyhow::Result<JsonValue> {
    match tool {
        "sources" => Ok(serde_json::json!({"sources": ["Reuters", "AP", "BBC", "HN"]})),
        "latest" => {
            let limit = params.get("limit").and_then(|v| v.as_u64()).unwrap_or(5) as usize;
            let items: Vec<serde_json::Value> = (1..=limit)
                .map(|i| serde_json::json!({"title": format!("Headline {} (stub)", i), "url": format!("https://example.com/{}", i)}))
                .collect();
            Ok(serde_json::json!({"items": items}))
        }
        "daily_brief" => {
            let cats = params.get("categories").and_then(|v| v.as_array()).map(|a| a.iter().filter_map(|x| x.as_str()).collect::<Vec<_>>()).unwrap_or_else(|| vec!["world", "tech"]);
            let md = format!("# News Brief\n\n- Categories: {}\n- {}\n", cats.join(", "), "Stub summary of top items.");
            Ok(serde_json::json!({"markdown": md}))
        }
        _ => Err(anyhow::anyhow!("unknown tool")),
    }
}

// --- Project tools (init directory) ---
async fn invoke_project(tool: &str, params: JsonValue) -> anyhow::Result<JsonValue> {
    match tool {
        "init" => {
            let name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let kind = params.get("kind").and_then(|v| v.as_str()).unwrap_or("dev"); // dev|games
            if name.is_empty() { anyhow::bail!("name required"); }
            if !(kind == "dev" || kind == "games") { anyhow::bail!("kind must be 'dev' or 'games'"); }
            let home = std::env::var("HOME").map_err(|_| anyhow::anyhow!("HOME not set"))?;
            let base = std::path::Path::new(&home).join(kind);
            // Validate base exists or create dev/games root as needed
            if !base.exists() { tokio::fs::create_dir_all(&base).await?; }
            // Only allow simple directory name (no path traversal)
            if name.contains('/') || name.contains("..") { anyhow::bail!("invalid project name"); }
            let dir = base.join(name);
            if dir.exists() { anyhow::bail!("project already exists"); }
            tokio::fs::create_dir_all(&dir).await?;
            // Write a minimal README
            let readme = dir.join("README.md");
            let md = format!("# {}\n\nInitialized by Foreman project.init in `{}`.\n", name, kind);
            let _ = tokio::fs::write(&readme, md).await;
            // Optionally git init
            if params.get("git").and_then(|v| v.as_bool()).unwrap_or(true) {
                // Best-effort: if git exists on PATH, run `git init`
                let _ = tokio::process::Command::new("git").arg("init").current_dir(&dir).output().await;
            }
            Ok(json!({"path": dir.to_string_lossy()}))
        }
        _ => Err(anyhow::anyhow!("unknown tool")),
    }
}

// --- Installer plan store and tools (in-process stub) ---
#[derive(Clone, Debug)]
struct InstallerPlan {
    id: String,
    manager: String,
    pkg: String,
    cmds: Vec<String>,
}

static mut INSTALLER_PLANS: Option<std::sync::Mutex<HashMap<String, InstallerPlan>>> = None;

fn plans() -> &'static std::sync::Mutex<HashMap<String, InstallerPlan>> {
    // Simple lazy init; safe here for single-process use
    unsafe {
        if INSTALLER_PLANS.is_none() {
            INSTALLER_PLANS = Some(std::sync::Mutex::new(HashMap::new()));
        }
        INSTALLER_PLANS.as_ref().unwrap()
    }
}

fn detect_manager(pkg: &str, provided: Option<&str>) -> String {
    if let Some(m) = provided { return m.to_string(); }
    if pkg.ends_with(".whl") || pkg.contains("/") { return "pip".into(); }
    // default to apt on Linux; this is a stub heuristic
    if cfg!(target_os = "linux") { "apt".into() } else { "cargo".into() }
}

fn build_commands(manager: &str, pkg: &str) -> Vec<String> {
    match manager {
        "apt" => vec![format!("sudo apt-get update"), format!("sudo apt-get install -y {}", pkg)],
        "snap" => vec![format!("sudo snap install {}", pkg)],
        "flatpak" => vec![format!("flatpak install -y {}", pkg)],
        "pip" => vec![format!("pip install {}", pkg)],
        "cargo" => vec![format!("cargo install {}", pkg)],
        _ => vec![format!("echo install {} via {} (unknown manager)", pkg, manager)],
    }
}

async fn invoke_installer(tool: &str, params: JsonValue) -> anyhow::Result<JsonValue> {
    match tool {
        "plan_install" => {
            let pkg = params.get("pkg").and_then(|v| v.as_str()).unwrap_or("");
            if pkg.is_empty() { anyhow::bail!("pkg required"); }
            let manager = detect_manager(pkg, params.get("manager").and_then(|v| v.as_str()));
            let cmds = build_commands(&manager, pkg);
            let id = Uuid::new_v4().to_string();
            let plan = InstallerPlan { id: id.clone(), manager: manager.clone(), pkg: pkg.to_string(), cmds: cmds.clone() };
            plans().lock().unwrap().insert(id.clone(), plan);
            Ok(json!({"plan_id": id, "manager": manager, "pkg": pkg, "commands": cmds }))
        }
        "explain_install" => {
            let pid = params.get("plan_id").and_then(|v| v.as_str()).unwrap_or("");
            let g = plans().lock().unwrap();
            let p = g.get(pid).ok_or_else(|| anyhow::anyhow!("unknown plan_id"))?;
            Ok(json!({
                "plan_id": p.id,
                "explain": format!("Install {} via {}. Source: distro/vendor repos where applicable; safety: dry-run first.", p.pkg, p.manager),
                "commands": p.cmds,
            }))
        }
        "dry_run" => {
            let pid = params.get("plan_id").and_then(|v| v.as_str()).unwrap_or("");
            let g = plans().lock().unwrap();
            let p = g.get(pid).ok_or_else(|| anyhow::anyhow!("unknown plan_id"))?;
            Ok(json!({"plan_id": p.id, "dry_run": true, "commands": p.cmds}))
        }
        "apply_install" => {
            // Token validation is enforced at API layer; here we just report success without executing.
            let pid = params.get("plan_id").and_then(|v| v.as_str()).unwrap_or("");
            let g = plans().lock().unwrap();
            let p = g.get(pid).ok_or_else(|| anyhow::anyhow!("unknown plan_id"))?;
            Ok(json!({"plan_id": p.id, "applied": false, "note": "execution stubbed in-core; external process integration later"}))
        }
        _ => Err(anyhow::anyhow!("unknown tool")),
    }
}

// Helper for approval prompt to fetch installer plan commands
pub fn installer_plan_commands(plan_id: &str) -> Option<Vec<String>> {
    let g = plans().lock().ok()?;
    let p = g.get(plan_id)?;
    Some(p.cmds.clone())
}
