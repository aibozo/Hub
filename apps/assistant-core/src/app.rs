use crate::config::Config;
use crate::gatekeeper::{ApprovalsStore, PolicyEngine, ProvenanceEngine};
use crate::memory::Memory;
use std::path::PathBuf;
use crate::system_map::SystemMapManager;
use crate::tools::ToolsManager;
use crate::realtime::RealtimeManager;
use crate::wake::{WakeSentinel, WakeOptions};
use crate::scheduler::{Scheduler, SchedulerConfig};
use crate::gatekeeper::ProposedAction;
use parking_lot::RwLock;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppHandles {
    pub policy: Arc<PolicyEngine>,
    pub approvals: ApprovalsStore,
    pub provenance: ProvenanceEngine,
    // Future wiring placeholders
    pub memory: Option<Memory>,
    pub system_map: SystemMapManager,
    pub tools: ToolsManager,
    pub mcp_client: (),
    pub scheduler: Scheduler,
    pub approval_prompt: Arc<RwLock<Option<EphemeralApproval>>>,
    pub realtime: RealtimeManager,
    pub wake: WakeSentinel,
}

#[derive(Clone)]
pub struct AppState {
    pub version: &'static str,
    pub config: Arc<RwLock<Config>>,
    pub handles: AppHandles,
}

pub type SharedState = Arc<AppState>;

impl AppState {
    pub async fn new(config: Config) -> SharedState {
        let policy = Arc::new(PolicyEngine::load_from_dir(std::path::PathBuf::from("config/policy.d")).unwrap_or_else(|_| PolicyEngine::default()));
        let approvals = ApprovalsStore::default();
        let provenance = ProvenanceEngine::default();
        // Initialize memory store
        let home = config.clone().home_dir();
        let base = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../"));
        let home_abs = if home.is_relative() { base.join(home) } else { home };
        let db_path = home_abs.join("sqlite.db");
        let migrations_dir = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/migrations"));
        let memory = match Memory::init(db_path.clone(), migrations_dir.clone()).await {
            Ok(m) => Some(m),
            Err(e) => {
                tracing::warn!(error=%e, ?db_path, "file-backed sqlite failed; falling back to in-memory");
                match Memory::init_in_memory(migrations_dir.clone()).await {
                    Ok(m) => Some(m),
                    Err(e2) => {
                        tracing::error!(error=%e2, "in-memory sqlite init failed");
                        eprintln!("[AppState::new] memory init error: {} (db_path={})", e2, db_path.display());
                        None
                    }
                }
            }
        };
        // Initialize System Map using the same home dir
        let system_map = SystemMapManager::new(&home_abs, memory.clone());
        // Load or scan map; do not fail hard on errors
        let _ = system_map.load_or_scan().await;

        // Load tool manifests
        let tools_dir = PathBuf::from("config/tools.d");
        let tools = ToolsManager::load_from_dir(&tools_dir);
        // Autostart MCP servers (best-effort)
        let tools_autostart = tools.clone();
        tokio::spawn(async move { tools_autostart.autostart().await; });
        let approval_prompt = Arc::new(RwLock::new(None));

        let sched_cfg = SchedulerConfig::load_from_file("config/schedules.toml");
        let tools_for_sched = tools.clone();
        let scheduler = Scheduler::new(sched_cfg, home_abs.clone(), memory.clone(), tools_for_sched);
        scheduler.clone().start();

        // Realtime manager (pass chats dir for context seeding)
        let chats_dir = system_map.map_path().parent().unwrap_or(std::path::Path::new(".")).join("chats");
        let realtime = RealtimeManager::new(tools.clone(), policy.clone(), approval_prompt.clone(), Some(chats_dir));
        // Wake sentinel
        let vc = config.voice.clone();
        let wake_opts = WakeOptions {
            phrase: vc.as_ref().and_then(|v| v.wake_phrase.clone()).unwrap_or_else(|| "hey vim".into()),
            enabled: vc.as_ref().and_then(|v| v.wake_enabled).unwrap_or(false),
            vad_sensitivity: vc.as_ref().and_then(|v| v.vad_sensitivity).unwrap_or(0.5),
            min_speech_ms: vc.as_ref().and_then(|v| v.min_speech_ms).unwrap_or(400),
            refractory_ms: vc.as_ref().and_then(|v| v.refractory_ms).unwrap_or(3000) as u64,
        };
        let wake = WakeSentinel::new(wake_opts);
        #[cfg(feature = "realtime-audio")]
        {
            let w = wake.clone();
            let rt = realtime.clone();
            tokio::spawn(async move { w.start_task(rt).await; });
        }

        Arc::new(AppState {
            version: env!("CARGO_PKG_VERSION"),
            config: Arc::new(RwLock::new(config)),
            handles: AppHandles { policy: policy.clone(), approvals, provenance, memory, system_map, tools, mcp_client: (), scheduler, approval_prompt, realtime, wake },
        })
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EphemeralApproval {
    pub id: String,
    pub title: String,
    pub action: ProposedAction,
    #[serde(default)]
    pub details: serde_json::Value,
}
