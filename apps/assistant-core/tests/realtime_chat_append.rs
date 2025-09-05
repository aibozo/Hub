use assistant_core::realtime::append_latest_chat_message as append_msg;
use std::path::PathBuf;

#[tokio::test]
async fn realtime_append_coalesces_by_role_and_preserves_turns() {
    // Use a unique temp dir under storage/
    let base = PathBuf::from("storage/tmp_test_chats");
    let _ = std::fs::create_dir_all(&base);
    let dir = base.join(format!("test-{}", uuid::Uuid::new_v4()));
    let _ = std::fs::create_dir_all(&dir);

    // Assistant streaming: should coalesce into one assistant message
    append_msg(&dir, "assistant", "Hello").await.unwrap();
    append_msg(&dir, "assistant", " world").await.unwrap();

    // User message ends the assistant turn
    append_msg(&dir, "user", "Ok").await.unwrap();

    // Next assistant turn should append a new assistant message
    append_msg(&dir, "assistant", "Again").await.unwrap();

    // Read latest chat file and assert roles/content
    let mut latest: Option<std::path::PathBuf> = None;
    if let Ok(mut rd) = tokio::fs::read_dir(&dir).await {
        while let Ok(Some(e)) = rd.next_entry().await { if e.file_type().await.unwrap().is_file() { latest = Some(e.path()); break; } }
    }
    let path = latest.expect("chat file");
    let bytes = tokio::fs::read(&path).await.unwrap();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let msgs = v.get("messages").and_then(|m| m.as_array()).cloned().unwrap();
    assert_eq!(msgs.len(), 3, "expected 3 messages (assistant, user, assistant), got {}", msgs.len());
    assert_eq!(msgs[0].get("role").and_then(|s| s.as_str()), Some("assistant"));
    assert_eq!(msgs[0].get("content").and_then(|s| s.as_str()), Some("Hello world"));
    assert_eq!(msgs[1].get("role").and_then(|s| s.as_str()), Some("user"));
    assert_eq!(msgs[2].get("role").and_then(|s| s.as_str()), Some("assistant"));
    assert_eq!(msgs[2].get("content").and_then(|s| s.as_str()), Some("Again"));
}

