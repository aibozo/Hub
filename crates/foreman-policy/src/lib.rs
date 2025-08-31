use serde::{Deserialize, Serialize};
use std::{fs, path::Path};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Limits {
    pub wall_time_sec: Option<u64>,
    pub cpu_percent: Option<u64>,
    pub mem_mb: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RedactionRule {
    pub pattern: String,
    pub replace: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PolicyRules {
    #[serde(default)]
    pub protect_paths: Vec<String>,
    #[serde(default)]
    pub write_whitelist: Vec<String>,
    #[serde(default)]
    pub require_approval: Vec<String>,
    #[serde(default)]
    pub env_allowlist: Vec<String>,
    #[serde(default)]
    pub limits: Limits,
    #[serde(default)]
    pub log_redactions: Vec<RedactionRule>,
}

impl PolicyRules {
    pub fn merge(mut self, other: PolicyRules) -> PolicyRules {
        self.protect_paths.extend(other.protect_paths);
        self.write_whitelist.extend(other.write_whitelist);
        self.require_approval.extend(other.require_approval);
        self.env_allowlist.extend(other.env_allowlist);
        if other.limits.wall_time_sec.is_some() { self.limits.wall_time_sec = other.limits.wall_time_sec; }
        if other.limits.cpu_percent.is_some() { self.limits.cpu_percent = other.limits.cpu_percent; }
        if other.limits.mem_mb.is_some() { self.limits.mem_mb = other.limits.mem_mb; }
        self.log_redactions.extend(other.log_redactions);
        self
    }
}

pub fn load_dir(dir: &Path) -> anyhow::Result<PolicyRules> {
    let mut entries: Vec<_> = fs::read_dir(dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map(|x| x == "yaml").unwrap_or(false))
        .collect();
    entries.sort_by_key(|e| e.file_name());
    let mut rules = PolicyRules::default();
    for e in entries {
        let text = fs::read_to_string(e.path())?;
        let part: PolicyRules = serde_yaml::from_str(&text)?;
        rules = rules.merge(part);
    }
    Ok(rules)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionRequest {
    pub command: String,
    #[serde(default)]
    pub writes: bool,
    #[serde(default)]
    pub paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DecisionKind { Allow, Warn, Hold }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Decision {
    pub kind: DecisionKind,
    pub reasons: Vec<String>,
}

pub fn evaluate(rules: &PolicyRules, req: &ActionRequest) -> Decision {
    // Require-approval keywords
    for needle in &rules.require_approval {
        if !needle.is_empty() && req.command.contains(needle) {
            return Decision { kind: DecisionKind::Hold, reasons: vec![format!("requires approval: {}", needle)] };
        }
    }

    if req.writes {
        // If any path is outside whitelist, hold
        if !req.paths.is_empty() && !rules.write_whitelist.is_empty() {
            let mut violations = vec![];
            'outer: for p in &req.paths {
                for allow in &rules.write_whitelist {
                    if p.starts_with(allow.trim_end_matches('/')) { continue 'outer; }
                }
                violations.push(p.clone());
            }
            if !violations.is_empty() {
                return Decision { kind: DecisionKind::Hold, reasons: vec![format!("write outside whitelist: {:?}", violations)] };
            }
        }
        return Decision { kind: DecisionKind::Warn, reasons: vec!["write operation".into()] };
    }

    Decision { kind: DecisionKind::Allow, reasons: vec!["read-only".into()] }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hold_on_require_keyword() {
        let rules = PolicyRules { require_approval: vec!["sudo".into()], ..Default::default() };
        let d = evaluate(&rules, &ActionRequest { command: "sudo rm -rf /".into(), writes: true, paths: vec!["/".into()] });
        assert!(matches!(d.kind, DecisionKind::Hold));
    }
    #[test]
    fn warn_on_write_in_whitelist() {
        let rules = PolicyRules { write_whitelist: vec!["~/".into()], ..Default::default() };
        let d = evaluate(&rules, &ActionRequest { command: "touch ~/file".into(), writes: true, paths: vec!["~/file".into()] });
        assert!(matches!(d.kind, DecisionKind::Warn));
    }
}
