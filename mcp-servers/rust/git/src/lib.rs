use anyhow::Result;
use serde_json::{json, Value as JsonValue};
use tokio::fs;
use tokio::process::Command;

pub async fn status(path: &str) -> Result<JsonValue> {
    let git_dir = format!("{}/.git", path.trim_end_matches('/'));
    let is_repo = fs::metadata(&git_dir).await.is_ok();
    if !is_repo {
        return Ok(json!({ "repo": false }));
    }
    // Run a simple git status --porcelain summary
    let out = Command::new("git").arg("-C").arg(path).arg("status").arg("--porcelain").output().await?;
    let ok = out.status.success();
    let changed = String::from_utf8_lossy(&out.stdout).lines().count();
    Ok(json!({ "repo": true, "ok": ok, "changed": changed }))
}

