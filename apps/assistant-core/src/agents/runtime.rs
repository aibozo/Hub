use crate::app::SharedState;
use crate::gatekeeper::{ProposedAction, Approval};
use serde_json::{json, Value as JsonValue};

pub fn spawn(state: SharedState, agent_id: String) {
    tokio::spawn(async move {
        if let Err(e) = run_agent(state.clone(), agent_id.clone()).await {
            if let Some(mem) = state.handles.memory.as_ref() {
                let _ = mem.store.append_event_for_agent(None, Some(&agent_id), "agent.error", Some(&json!({"error": e.to_string()}))).await;
            }
            let _ = super_set_status(&state, &agent_id, "NeedsAttention").await;
        }
    });
}

async fn super_set_status(state: &SharedState, id: &str, status: &str) -> anyhow::Result<()> {
    if let Some(mem) = state.handles.memory.as_ref() {
        mem.store.update_agent_status(id, status).await?;
        state.handles.agents.set_status(id, super::AgentStatus::from_str(status));
        let task_id = mem.store.get_agent(id).await?.map(|a| a.task_id);
        let _ = mem.store.append_event_for_agent(task_id, Some(id), &format!("agent.{}", status.to_lowercase()), None).await;
        Ok(())
    } else { anyhow::bail!("memory not initialized") }
}

async fn run_agent(state: SharedState, agent_id: String) -> anyhow::Result<()> {
    let mem = state.handles.memory.as_ref().ok_or_else(|| anyhow::anyhow!("memory not initialized"))?;
    let agent = mem.store.get_agent(&agent_id).await?.ok_or_else(|| anyhow::anyhow!("agent not found"))?;
    // Base storage root
    let storage_root = state.handles.system_map.map_path().parent().unwrap_or(std::path::Path::new("storage")).to_path_buf();
    let abs_root = {
        let p = std::path::Path::new(&agent.root_dir);
        if p.is_absolute() { p.to_path_buf() } else { storage_root.join(p) }
    };
    tokio::fs::create_dir_all(&abs_root).await.ok();
    let _ = mem.store.append_event_for_agent(Some(agent.task_id), Some(&agent_id), "agent.runtime.start", Some(&json!({"root": abs_root})) ).await;
    // Try Codex planning (best-effort)
    if let Some(plan_info) = try_codex_plan(&state, &agent_id, &agent.title, &abs_root).await {
        let _ = mem.store.append_event_for_agent(Some(agent.task_id), Some(&agent_id), "agent.codex.session", Some(&plan_info)).await;
    }
    super_set_status(&state, &agent_id, "Running").await.ok();

    // Plan: trivial edit for MVP (write CTR_HELLO.txt)
    let hello_path = abs_root.join("CTR_HELLO.txt");
    let hello_path_str = hello_path.to_string_lossy().to_string();
    let action = ProposedAction { command: "apply_patch".into(), writes: true, paths: vec![hello_path_str.clone()], intent: Some("write hello file".into()) };
    // Policy
    let decision = state.handles.policy.evaluate(&action);
    if !matches!(decision.kind, crate::gatekeeper::PolicyDecisionKind::Allow) {
        // Auto-approve if agent's auto_approval_level >= 2
        if agent.auto_approval_level >= 2 {
            let approval: Approval = state.handles.approvals.create(action.clone());
            let _ = mem.store.append_event_for_agent(Some(agent.task_id), Some(&agent_id), "agent.approval.auto", Some(&json!({"id": approval.id}))).await;
            let _ = state.handles.approvals.approve(&approval.id);
        } else {
            // Raise ephemeral prompt and pause
            let prompt = crate::app::EphemeralApproval { id: uuid::Uuid::new_v4().to_string(), title: "Agent action requires approval".into(), action: action.clone(), details: json!({"reasons": decision.reasons}) };
            *state.handles.approval_prompt.write() = Some(prompt);
            let _ = mem.store.append_event_for_agent(Some(agent.task_id), Some(&agent_id), "agent.approval.required", Some(&json!({"paths": action.paths}))).await;
            super_set_status(&state, &agent_id, "NeedsAttention").await?;
            return Ok(()); // stop until resumed
        }
    }

    // Apply patch
    let params = json!({"edits": [{"path": hello_path_str, "content": "hello from CTR\n", "create_dirs": true}]});
    let res = state.handles.tools.invoke("patch", "apply", params).await?;
    let _ = mem.store.append_event_for_agent(Some(agent.task_id), Some(&agent_id), "agent.apply.ok", Some(&res)).await;

    // Validate: file exists
    let exists = tokio::fs::metadata(&hello_path).await.is_ok();
    if !exists { anyhow::bail!("expected file missing after apply"); }

    // Commit if git repo
    let status = state.handles.tools.invoke("git", "status", json!({"path": abs_root.to_string_lossy()})).await.unwrap_or_else(|_| json!({"repo": false}));
    if status.get("repo").and_then(|v| v.as_bool()) == Some(true) {
        let _ = state.handles.tools.invoke("git", "add", json!({"path": abs_root.to_string_lossy(), "patterns": ["."]})).await.ok();
        let _ = state.handles.tools.invoke("git", "commit", json!({"path": abs_root.to_string_lossy(), "message": "CTR: add hello"})).await.ok();
    }

    // Done
    super_set_status(&state, &agent_id, "Done").await?;
    let _ = mem.store.append_event_for_agent(Some(agent.task_id), Some(&agent_id), "agent.done", None).await;
    Ok(())
}

async fn try_codex_plan(state: &SharedState, agent_id: &str, title: &str, abs_root: &std::path::Path) -> Option<JsonValue> {
    // Build a concise planning prompt
    let prompt = format!("Plan initial steps for agent '{}' working under '{}'. Propose a first safe edit set.", title, abs_root.display());
    let params = json!({"prompt": prompt, "repo": abs_root.to_string_lossy(), "config": {"sessionMeta": {"agentId": agent_id}}});
    match state.handles.tools.invoke("codex", "new", params).await {
        Ok(v) => Some(v),
        Err(e) => {
            if let Some(mem) = state.handles.memory.as_ref() {
                let _ = mem.store.append_event_for_agent(None, Some(agent_id), "agent.codex.unavailable", Some(&json!({"error": e.to_string()}))).await;
            }
            None
        }
    }
}
