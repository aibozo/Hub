use anyhow::Result;
use serde_json::{json, Value as JsonValue};
use tokio::fs;

pub async fn stat(path: &str) -> Result<JsonValue> {
    let meta = fs::metadata(path).await?;
    let is_dir = meta.is_dir();
    let is_file = meta.is_file();
    let size = if is_file { Some(meta.len()) } else { None };
    Ok(json!({ "is_dir": is_dir, "is_file": is_file, "size": size }))
}

pub async fn list(path: &str) -> Result<JsonValue> {
    let mut entries: Vec<String> = vec![];
    let mut dir = fs::read_dir(path).await?;
    while let Ok(Some(ent)) = dir.next_entry().await {
        if let Ok(name) = ent.file_name().into_string() { entries.push(name); }
    }
    entries.sort();
    Ok(json!({ "entries": entries }))
}

pub async fn read(path: &str) -> Result<JsonValue> {
    let content = fs::read_to_string(path).await?;
    Ok(json!({ "content": content }))
}

