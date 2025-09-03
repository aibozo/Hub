use assistant_core::memory::Memory;
use sqlx::Row;
use std::path::PathBuf;

#[tokio::test]
async fn memory_agent_crud() {
    let migrations_dir = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/migrations"));
    let mem = Memory::init_in_memory(migrations_dir).await.expect("init memory");
    // Create a task
    let task = mem.store.create_task("Test Task", "open", None).await.expect("task");
    // Create agent
    let agent_id = "agent-test-1";
    let agent = mem.store
        .create_agent(agent_id, task.id, "Agent Title", "Draft", ".", Some("gpt-4.1"), None, 1, None)
        .await
        .expect("create agent");
    assert_eq!(agent.id, agent_id);
    assert_eq!(agent.status, "Draft");
    // Update status
    mem.store.update_agent_status(agent_id, "Running").await.expect("status");
    let fetched = mem.store.get_agent(agent_id).await.expect("get agent").expect("some");
    assert_eq!(fetched.status, "Running");
    // List
    let list = mem.store.list_agents(10).await.expect("list");
    assert!(list.iter().any(|a| a.id == agent_id));
}

#[tokio::test]
async fn memory_event_linking() {
    let migrations_dir = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/migrations"));
    let mem = Memory::init_in_memory(migrations_dir).await.expect("init memory");
    let task = mem.store.create_task("T", "open", None).await.expect("task");
    let agent_id = "agent-test-2";
    let _ = mem.store
        .create_agent(agent_id, task.id, "A", "Draft", ".", None, None, 1, None)
        .await
        .expect("create agent");
    // Append event scoped to agent
    let payload = serde_json::json!({"hello":"world"});
    let _eid = mem.store
        .append_event_for_agent(Some(task.id), Some(agent_id), "agent.started", Some(&payload))
        .await
        .expect("append");
    // Fetch recent by agent
    let evs = mem.store.get_recent_events_by_agent(agent_id, 10).await.expect("events");
    assert!(evs.iter().any(|e| e.kind == "agent.started"));
}

#[tokio::test]
async fn memory_artifact_linking() {
    let migrations_dir = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/migrations"));
    let mem = Memory::init_in_memory(migrations_dir).await.expect("init memory");
    let task = mem.store.create_task("T", "open", None).await.expect("task");
    let agent_id = "agent-test-3";
    let _ = mem.store
        .create_agent(agent_id, task.id, "A", "Draft", ".", None, None, 1, None)
        .await
        .expect("create agent");
    // Create artifact and link
    let art_id = mem.store.create_artifact(task.id, std::path::Path::new("/tmp/x.txt"), Some("text/plain"), None).await.expect("artifact");
    mem.store.link_artifact_agent(art_id, agent_id).await.expect("link");
    // Verify via direct query
    let row = sqlx::query("SELECT agent_id FROM Artifact WHERE id = ?1").bind(art_id).fetch_one(mem.store.pool()).await.expect("query");
    let aid: Option<String> = row.get("agent_id");
    assert_eq!(aid.as_deref(), Some(agent_id));
}
