use crate::gatekeeper::{PolicyDecision, PolicyEngine, ProposedAction};

#[derive(Clone, Default)]
pub struct McpClient {}

impl McpClient {
    pub fn new() -> Self { Self {} }

    pub fn preflight_check(&self, gate: &PolicyEngine, action: &ProposedAction) -> PolicyDecision {
        gate.evaluate(action)
    }
}
