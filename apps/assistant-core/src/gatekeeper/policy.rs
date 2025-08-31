use foreman_policy as fp;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProposedAction {
    pub command: String,
    #[serde(default)]
    pub writes: bool,
    #[serde(default)]
    pub paths: Vec<String>,
    #[serde(default)]
    pub intent: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PolicyDecisionKind { Allow, Warn, Hold }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyDecision {
    pub kind: PolicyDecisionKind,
    pub reasons: Vec<String>,
}

pub struct PolicyEngine {
    rules: fp::PolicyRules,
}

impl PolicyEngine {
    pub fn default() -> Self { Self { rules: fp::PolicyRules::default() } }
    pub fn load_from_dir(dir: PathBuf) -> anyhow::Result<Self> {
        let rules = fp::load_dir(&dir)?;
        Ok(Self { rules })
    }

    pub fn evaluate(&self, action: &ProposedAction) -> PolicyDecision {
        let req = fp::ActionRequest { command: action.command.clone(), writes: action.writes, paths: action.paths.clone() };
        let d = fp::evaluate(&self.rules, &req);
        PolicyDecision { kind: match d.kind { fp::DecisionKind::Allow => PolicyDecisionKind::Allow, fp::DecisionKind::Warn => PolicyDecisionKind::Warn, fp::DecisionKind::Hold => PolicyDecisionKind::Hold }, reasons: d.reasons }
    }
}
