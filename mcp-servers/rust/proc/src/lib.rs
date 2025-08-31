use anyhow::Result;
use serde_json::{json, Value as JsonValue};
use tokio::fs;

pub async fn list() -> Result<JsonValue> {
    // List processes by scanning /proc for numeric dirs
    let mut pids: Vec<i32> = vec![];
    let mut dir = fs::read_dir("/proc").await?;
    while let Ok(Some(ent)) = dir.next_entry().await {
        if let Ok(name) = ent.file_name().into_string() {
            if let Ok(pid) = name.parse::<i32>() { pids.push(pid); }
        }
    }
    pids.sort();
    Ok(json!({ "pids": pids }))
}

