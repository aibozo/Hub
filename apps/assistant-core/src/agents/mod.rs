use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AgentStatus { Draft, Running, NeedsAttention, Paused, Blocked, Done, Aborted }

impl AgentStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            AgentStatus::Draft => "Draft",
            AgentStatus::Running => "Running",
            AgentStatus::NeedsAttention => "NeedsAttention",
            AgentStatus::Paused => "Paused",
            AgentStatus::Blocked => "Blocked",
            AgentStatus::Done => "Done",
            AgentStatus::Aborted => "Aborted",
        }
    }
    pub fn from_str(s: &str) -> Self {
        match s {
            "Running" => AgentStatus::Running,
            "NeedsAttention" => AgentStatus::NeedsAttention,
            "Paused" => AgentStatus::Paused,
            "Blocked" => AgentStatus::Blocked,
            "Done" => AgentStatus::Done,
            "Aborted" => AgentStatus::Aborted,
            _ => AgentStatus::Draft,
        }
    }
}

#[derive(Clone, Default)]
pub struct AgentsSupervisor {
    states: Arc<RwLock<HashMap<String, AgentStatus>>>,
}

impl AgentsSupervisor {
    pub fn new() -> Self { Self { states: Arc::new(RwLock::new(HashMap::new())) } }
    pub fn set_status(&self, id: &str, status: AgentStatus) { self.states.write().insert(id.to_string(), status); }
    pub fn get_status(&self, id: &str) -> Option<AgentStatus> { self.states.read().get(id).copied() }
    pub fn snapshot(&self) -> Vec<(String, AgentStatus)> { self.states.read().iter().map(|(k,v)| (k.clone(), *v)).collect() }
}

pub mod runtime;
