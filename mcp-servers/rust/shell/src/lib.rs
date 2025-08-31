use anyhow::{Result, bail};
use serde::{Deserialize};
use serde_json::{json, Value as JsonValue};
use once_cell::sync::OnceCell;
use std::path::PathBuf;

pub async fn list_dir(path: &str) -> Result<JsonValue> {
    let mut entries: Vec<String> = vec![];
    let mut dir = tokio::fs::read_dir(path).await?;
    while let Ok(Some(ent)) = dir.next_entry().await {
        if let Ok(name) = ent.file_name().into_string() { entries.push(name); }
    }
    entries.sort();
    Ok(json!({ "entries": entries }))
}

pub async fn read_file(path: &str) -> Result<JsonValue> {
    let data = tokio::fs::read_to_string(path).await?;
    Ok(json!({ "content": data }))
}

pub async fn which_cmd(cmd: &str) -> Result<JsonValue> {
    let path = which::which(cmd).ok().map(|p| p.to_string_lossy().to_string());
    Ok(json!({ "path": path }))
}

pub async fn exec(cmd: &str, args: &[String], wait: bool) -> Result<JsonValue> {
    // Enforce a strict whitelist for execution
    validate_exec(cmd, args)?;
    // Execute
    let mut c = tokio::process::Command::new(cmd);
    c.args(args);
    if wait {
        let out = c.output().await?;
        let ok = out.status.success();
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let stderr = String::from_utf8_lossy(&out.stderr).to_string();
        Ok(json!({ "ok": ok, "code": out.status.code(), "stdout": stdout, "stderr": stderr }))
    } else {
        use tokio::process::Command as TokioCommand;
        use std::process::Stdio;
        let mut c = TokioCommand::new(cmd);
        c.args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        let mut child = c.spawn()?;
        let pid = child.id().unwrap_or(0) as i64;
        // Detach: reap in background to avoid zombies
        tokio::spawn(async move {
            let _ = child.wait().await;
        });
        Ok(json!({ "ok": true, "pid": pid, "cmd": cmd, "args": args }))
    }
}

pub fn normalize_path(p: &str) -> PathBuf {
    let pb = PathBuf::from(p);
    pb
}

#[derive(Debug, Clone, Deserialize)]
struct ShellArgsRule {
    #[serde(default)]
    count: Option<usize>,
    #[serde(default)]
    min_count: Option<usize>,
    #[serde(default)]
    max_count: Option<usize>,
    #[serde(default)]
    path_prefixes: Option<Vec<String>>, // all args must start with any of these prefixes
    #[serde(default)]
    starts_with_tokens: Option<Vec<String>>, // e.g., ["-applaunch"]
    #[serde(default)]
    digits_at: Option<usize>, // index that must be numeric
}

#[derive(Debug, Clone, Deserialize)]
struct ShellRule { #[serde(rename = "match")] r#match: String, #[serde(default)] args: Option<ShellArgsRule> }

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
    if rules.is_empty() {
        rules.push(ShellRule { r#match: "mgba-qt".into(), args: Some(ShellArgsRule { count: Some(1), min_count: None, max_count: None, path_prefixes: Some(vec!["/home/kil/games/roms".into()]), starts_with_tokens: None, digits_at: None }) });
        rules.push(ShellRule { r#match: "/home/kil/games/emulators/melonDS-x86_64.AppImage".into(), args: Some(ShellArgsRule { count: Some(0), min_count: None, max_count: None, path_prefixes: None, starts_with_tokens: None, digits_at: None }) });
        rules.push(ShellRule { r#match: "melonDS-x86_64.AppImage".into(), args: Some(ShellArgsRule { count: Some(0), min_count: None, max_count: None, path_prefixes: None, starts_with_tokens: None, digits_at: None }) });
    }
    let _ = SHELL_RULES.set(rules.clone());
    rules
}

fn trim_unquote(s: &str) -> String {
    let t = s.trim();
    if (t.starts_with('"') && t.ends_with('"')) || (t.starts_with('\'') && t.ends_with('\'')) { t[1..t.len().saturating_sub(1)].to_string() } else { t.to_string() }
}
fn basename(s: &str) -> &str { std::path::Path::new(s).file_name().and_then(|x| x.to_str()).unwrap_or(s) }

fn validate_exec(cmd: &str, args: &[String]) -> Result<()> {
    let rules = load_shell_rules();
    let cmd_name = basename(cmd);
    let n_args: Vec<String> = args.iter().map(|a| trim_unquote(a)).collect();
    for r in rules.iter() {
        if r.r#match == cmd || r.r#match == cmd_name {
            if let Some(a) = &r.args {
                if let Some(c) = a.count { if n_args.len() != c { continue; } }
                if let Some(minc) = a.min_count { if n_args.len() < minc { continue; } }
                if let Some(maxc) = a.max_count { if n_args.len() > maxc { continue; } }
                if let Some(prefixes) = &a.path_prefixes {
                    if !n_args.iter().all(|s| prefixes.iter().any(|p| s.starts_with(p))) { continue; }
                }
                if let Some(tokens) = &a.starts_with_tokens {
                    if n_args.len() < tokens.len() { continue; }
                    let mut ok = true;
                    for (i, t) in tokens.iter().enumerate() {
                        if &n_args[i] != t { ok = false; break; }
                    }
                    if !ok { continue; }
                }
                if let Some(ix) = a.digits_at {
                    if n_args.get(ix).map(|s| s.chars().all(|c| c.is_ascii_digit())).unwrap_or(false) == false { continue; }
                }
            }
            return Ok(());
        }
    }
    bail!("command not whitelisted: {} (see config/policy.d/*shell* for allowlist)", cmd)
}
