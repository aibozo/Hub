pub mod model;
pub mod scan;
pub mod digest;

use crate::memory::Memory;
use crate::system_map::digest::compute_digest;
use parking_lot::RwLock;
use serde_json::json;
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Clone)]
pub struct SystemMapManager {
    inner: Arc<Inner>,
}

struct Inner {
    map_path: PathBuf,
    map: RwLock<Option<model::SystemMap>>,
    digest: RwLock<String>,
    memory: Option<Memory>,
}

impl SystemMapManager {
    pub fn new(base_dir: &Path, memory: Option<Memory>) -> Self {
        let map_path = base_dir.join("map.json");
        Self {
            inner: Arc::new(Inner {
                map_path,
                map: RwLock::new(None),
                digest: RwLock::new(String::new()),
                memory,
            }),
        }
    }

    pub fn map_path(&self) -> &Path { &self.inner.map_path }

    pub fn get_map(&self) -> Option<model::SystemMap> {
        self.inner.map.read().clone()
    }

    pub fn get_digest(&self) -> String {
        self.inner.digest.read().clone()
    }

    pub async fn load_or_scan(&self) -> anyhow::Result<()> {
        // Try load from disk; fall back to scan and persist
        if let Ok(text) = tokio::fs::read_to_string(&self.inner.map_path).await {
            if let Ok(map) = serde_json::from_str::<model::SystemMap>(&text) {
                let digest = compute_digest(&map);
                *self.inner.map.write() = Some(map);
                *self.inner.digest.write() = digest;
                return Ok(());
            }
        }
        let map = scan::scan_system().await;
        self.update_with(map).await
    }

    pub async fn refresh(&self) -> anyhow::Result<()> {
        let map = scan::scan_system().await;
        self.update_with(map).await
    }

    pub async fn update_with(&self, new_map: model::SystemMap) -> anyhow::Result<()> {
        // Compare with existing
        let mut changed = true;
            if let Some(old) = self.inner.map.read().as_ref() {
            let old_json = serde_json::to_string(old)?;
            let new_json = serde_json::to_string(&new_map)?;
            changed = old_json != new_json;
        }

        // Persist
        if let Some(parent) = self.inner.map_path.parent() { tokio::fs::create_dir_all(parent).await.ok(); }
        let data = serde_json::to_vec_pretty(&new_map)?;
        tokio::fs::write(&self.inner.map_path, data).await?;

        // Update in-memory state
        let digest = compute_digest(&new_map);
        *self.inner.map.write() = Some(new_map.clone());
        *self.inner.digest.write() = digest;

        // Emit event if changed
        if changed {
            if let Some(mem) = self.inner.memory.as_ref() {
                let _ = mem
                    .store
                    .append_event(
                        None,
                        "system_map:updated",
                        Some(&json!({
                            "scanned_at": new_map.scanned_at,
                            "os": new_map.os.name,
                            "runtimes": new_map.runtimes,
                        })),
                    )
                    .await;
            }
        }
        Ok(())
    }

    pub fn resolve_uri(&self, uri: &str) -> Option<serde_json::Value> {
        let map = self.inner.map.read();
        let map = map.as_ref()?;
        match uri {
            "map://packages" => Some(json!({
                "package_managers": map.package_managers,
                "apps": map.apps,
            })),
            "map://emulators" => Some(json!({
                "emulators": map.apps.iter().filter(|a| a.contains("emu")).collect::<Vec<_>>()
            })),
            "map://worktrees" => Some(json!({
                "vcs": map.dev_env.vcs,
            })),
            _ => None,
        }
    }
}

pub use model::SystemMap;
