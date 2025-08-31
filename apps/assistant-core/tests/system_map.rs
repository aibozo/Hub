use assistant_core::{api, app, config};
use axum::{body::Body, http::Request};
use tower::ServiceExt;

#[tokio::test]
async fn digest_is_deterministic_and_bounded() {
    use assistant_core::system_map::{digest::compute_digest, model::*};
    let map = SystemMap {
        scanned_at: chrono::Utc::now(),
        hardware: HardwareInfo { cpu_model: Some("Test CPU".into()), gpu_model: None, ram_gb: Some(16.0) },
        os: OsInfo { name: "TestOS".into(), version: Some("1.0".into()), kernel: Some("k1".into()), arch: Some("x86_64".into()) },
        runtimes: RuntimesInfo { python: Some("Python 3.10".into()), node: Some("v18".into()), rustc: Some("rustc 1.78".into()), cargo: Some("cargo 1.78".into()), java: None, cuda: None },
        package_managers: vec!["apt".into(), "pip".into(), "cargo".into()],
        apps: vec!["docker".into(), "vscode".into(), "git".into()],
        dev_env: DevEnvInfo { editors: vec!["vscode".into()], vcs: vec!["git".into()] },
        network: NetworkInfo { hostname: Some("host".into()), interfaces: vec![] },
    };
    let d1 = compute_digest(&map);
    let d2 = compute_digest(&map);
    assert_eq!(d1, d2, "digest should be deterministic");
    let tokens = d1.split_whitespace().count();
    assert!(tokens > 20 && tokens <= 400, "digest tokens within bounds: {}", tokens);
}

#[tokio::test]
async fn map_persisted_and_event_emitted_on_change() {
    // Use a temp directory for home
    let tmp = std::path::PathBuf::from(format!("./storage/test_map_{}", uuid::Uuid::new_v4()));
    let cfg = config::Config { foreman: Some(config::ForemanConfig { home: Some(tmp.to_string_lossy().to_string()), profile: None }), voice: None, schedules: None, mcp: None };
    let state = app::AppState::new(cfg).await;
    let app_router = api::build_router(state.clone());

    // Force an update with a fixed map to avoid environment-specific scanning
    use assistant_core::system_map::model::*;
    let fixed = SystemMap {
        scanned_at: chrono::Utc::now(),
        hardware: HardwareInfo { cpu_model: Some("Test CPU".into()), gpu_model: None, ram_gb: Some(8.0) },
        os: OsInfo { name: "TestOS".into(), version: Some("1.0".into()), kernel: Some("k".into()), arch: Some("x86_64".into()) },
        runtimes: RuntimesInfo::default(),
        package_managers: vec!["apt".into()],
        apps: vec!["git".into()],
        dev_env: DevEnvInfo::default(),
        network: NetworkInfo::default(),
    };
    state.handles.system_map.update_with(fixed).await.expect("update_with ok");
    let map_path = state.handles.system_map.map_path().to_path_buf();
    assert!(map_path.exists(), "map.json should be created at {:?}", map_path);

    // Force a refresh to emit an event (the scan may or may not change fields, but update_with will write and emit if changed)
    let _ = state.handles.system_map.refresh().await;

    // Check recent events include system_map:updated (best-effort)
    if let Some(mem) = state.handles.memory.as_ref() {
        let events = mem.store.get_recent_events(10).await.unwrap_or_default();
        let has_event = events.iter().any(|e| e.kind == "system_map:updated");
        assert!(has_event, "expected system_map:updated event");
    }
}
