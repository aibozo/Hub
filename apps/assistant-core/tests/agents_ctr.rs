use assistant_core::{app, agents, config};
use std::path::PathBuf;

async fn setup_state() -> app::SharedState {
    app::AppState::new(config::Config::default()).await
}

#[tokio::test]
async fn ctr_runs_all_green_with_auto_approval() {
    let state = setup_state().await;
    let mem = state.handles.memory.as_ref().expect("memory");
    let task = mem.store.create_task("CTR Test", "open", None).await.expect("task");
    let root_rel = format!("dev/ctr_demo_{}", uuid::Uuid::new_v4());
    let agent_id = format!("agent-{}", uuid::Uuid::new_v4());
    let _agent = mem
        .store
        .create_agent(
            &agent_id,
            task.id,
            "CTR Agent",
            "Draft",
            &root_rel,
            Some("gpt-4.1"),
            None,
            2, // aggressive auto-approval
            None,
        )
        .await
        .expect("create agent");

    // Start runtime
    agents::runtime::spawn(state.clone(), agent_id.clone());

    // Wait for completion
    let mut tries = 0;
    let status_done = loop {
        let a = mem.store.get_agent(&agent_id).await.unwrap().unwrap();
        if a.status == "Done" { break true; }
        tries += 1;
        if tries > 200 { break false; }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    };
    assert!(status_done, "agent did not reach Done status");

    // File exists under storage/<root_rel>/CTR_HELLO.txt
    let storage_root = state.handles.system_map.map_path().parent().unwrap().to_path_buf();
    let path = storage_root.join(&root_rel).join("CTR_HELLO.txt");
    assert!(std::fs::metadata(&path).is_ok(), "expected file missing: {}", path.display());

    // Event present
    let evs = mem.store.get_recent_events_by_agent(&agent_id, 10).await.unwrap();
    assert!(evs.iter().any(|e| e.kind == "agent.done"));
    // If Codex is not available, ensure unavailability is logged (best-effort)
    let evs2 = mem.store.get_recent_events_by_agent(&agent_id, 50).await.unwrap();
    assert!(evs2.iter().any(|e| e.kind == "agent.codex.unavailable") || evs2.iter().any(|e| e.kind == "agent.codex.session"));
}

#[tokio::test]
async fn ctr_escalates_then_pauses_without_auto_approval() {
    let state = setup_state().await;
    let mem = state.handles.memory.as_ref().expect("memory");
    let task = mem.store.create_task("CTR Test 2", "open", None).await.expect("task");
    let root_rel = format!("dev/ctr_demo_{}", uuid::Uuid::new_v4());
    let agent_id = format!("agent-{}", uuid::Uuid::new_v4());
    let _agent = mem
        .store
        .create_agent(
            &agent_id,
            task.id,
            "CTR Agent",
            "Draft",
            &root_rel,
            Some("gpt-4.1"),
            None,
            1, // default lane → requires manual approval for apply_patch
            None,
        )
        .await
        .expect("create agent");

    agents::runtime::spawn(state.clone(), agent_id.clone());

    // Wait until it pauses with NeedsAttention
    let mut tries = 0;
    let paused = loop {
        let a = mem.store.get_agent(&agent_id).await.unwrap().unwrap();
        if a.status == "NeedsAttention" { break true; }
        tries += 1;
        if tries > 200 { break false; }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    };
    assert!(paused, "agent did not reach NeedsAttention status");

    // Ephemeral prompt should be set
    let ep = state.handles.approval_prompt.read().clone();
    assert!(ep.is_some(), "expected ephemeral approval prompt");
    if let Some(p) = ep { assert_eq!(p.action.command, "apply_patch"); }
}
