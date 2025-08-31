use super::policy::ProposedAction;
use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

pub type ApprovalId = String;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ApprovalStatus { Pending, Approved, Denied }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Approval {
    pub id: ApprovalId,
    pub created_at: DateTime<Utc>,
    pub status: ApprovalStatus,
    pub action: ProposedAction,
    pub token: Option<String>,
}

#[derive(Default)]
pub struct ApprovalsStoreInner {
    items: HashMap<ApprovalId, Approval>,
}

#[derive(Clone, Default)]
pub struct ApprovalsStore(Arc<RwLock<ApprovalsStoreInner>>);

impl ApprovalsStore {
    pub fn create(&self, action: ProposedAction) -> Approval {
        let id = Uuid::new_v4().to_string();
        let approval = Approval { id: id.clone(), created_at: Utc::now(), status: ApprovalStatus::Pending, action, token: None };
        self.0.write().items.insert(id.clone(), approval.clone());
        approval
    }

    pub fn list(&self) -> Vec<Approval> {
        self.0.read().items.values().cloned().collect()
    }

    pub fn approve(&self, id: &str) -> Option<Approval> {
        let mut g = self.0.write();
        let a = g.items.get_mut(id)?;
        a.status = ApprovalStatus::Approved;
        a.token = Some(Uuid::new_v4().to_string());
        Some(a.clone())
    }

    pub fn deny(&self, id: &str) -> Option<Approval> {
        let mut g = self.0.write();
        let a = g.items.get_mut(id)?;
        a.status = ApprovalStatus::Denied;
        a.token = None;
        Some(a.clone())
    }

    pub fn get(&self, id: &str) -> Option<Approval> {
        self.0.read().items.get(id).cloned()
    }

    pub fn validate_token(&self, id: &str, token: &str) -> bool {
        self.0
            .read()
            .items
            .get(id)
            .map(|a| matches!(a.status, ApprovalStatus::Approved) && a.token.as_deref() == Some(token))
            .unwrap_or(false)
    }
}

