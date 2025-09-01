mod api;
mod app;
mod config;
mod gatekeeper;
mod memory;
mod mcp_client;
mod scheduler;
mod telemetry;
mod system_map;
mod tools;
mod metrics;
mod realtime;
mod realtime_audio;
mod wake;
mod stt;
mod prompt;

use anyhow::Context;
use axum::Router;
use std::net::SocketAddr;
use tracing::{error, info};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    telemetry::init();

    let (cfg, cfg_path) = config::Config::load().context("loading config/foreman.toml")?;
    info!(?cfg_path, "config loaded");

    let state = app::AppState::new(cfg).await;
    let router: Router = api::build_router(state);

    let addr: SocketAddr = std::env::var("FOREMAN_BIND")
        .unwrap_or_else(|_| "127.0.0.1:6061".to_string())
        .parse()
        .context("invalid FOREMAN_BIND address")?;

    info!(%addr, version = env!("CARGO_PKG_VERSION"), "assistant-core listening");

    let server = axum::serve(
        tokio::net::TcpListener::bind(addr).await?,
        router.into_make_service_with_connect_info::<SocketAddr>(),
    );

    let graceful = server.with_graceful_shutdown(async move {
        let _ = tokio::signal::ctrl_c().await;
        info!("ctrl-c received; shutting down");
    });

    if let Err(e) = graceful.await {
        error!(error = %e, "server error");
    }

    Ok(())
}
