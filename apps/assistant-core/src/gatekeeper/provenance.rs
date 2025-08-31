use super::policy::ProposedAction;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExplainCard {
    pub id: String,
    pub summary: String,
    pub command: String,
    pub sources: Vec<String>,
    pub notes: Vec<String>,
}

#[derive(Default, Clone)]
pub struct ProvenanceEngine;

impl ProvenanceEngine {
    pub fn explain(&self, id: &str, action: &ProposedAction) -> ExplainCard {
        let mut notes = vec![];
        if action.command.contains("apt ") { notes.push("Package manager: apt (dry-run available via -s)".into()); }
        if action.command.contains("pip ") { notes.push("Python package install; consider --dry-run/no-deps and hashes".into()); }
        if action.command.contains("sudo") { notes.push("Sudo escalation included; verify necessity".into()); }
        ExplainCard {
            id: id.to_string(),
            summary: "Proposed action requires review".into(),
            command: action.command.clone(),
            sources: vec![],
            notes,
        }
    }
}

