# PR-0002 — assistant-core Skeleton

Summary: Scaffold the Rust `assistant-core` binary with config loading, telemetry, basic module layout, and a minimal WS/HTTP control API stub. No tool calls yet.

Dependencies: PR-0001.

Deliverables

- `apps/assistant-core/` with:
  - `src/main.rs`: parse config, init telemetry, start HTTP/WS server, advertise version.
  - `src/app.rs`: wiring struct holding handles to subsystems.
  - `src/api.rs`: Axum/Tungstenite routes: `GET /health`, `GET /metrics` (stub), `WS /control` (echo).
  - `src/config.rs`: load `config/foreman.toml` with defaults and env overrides.
  - `src/telemetry.rs`: `tracing_subscriber` setup and Prometheus exporter.
  - `src/scheduler.rs`: cron-like stub with no-op jobs.
  - `src/mcp_client.rs`: type skeleton only (no connections).
- `src/app.rs` exposes handles for: policy, memory, system map, mcp client, scheduler.
- Feature flag `net-tests` for networked tests (off by default).

Implementation Notes

- Use `tokio` runtime; spawn HTTP server on `127.0.0.1:6061` by default (configurable).
- Expose `GET /health` => `{status:"ok", version}` and `GET /ready` => 200 if config loaded.
- Leave TODO hooks for policy/memory/system-map to be wired in later PRs.

Wiring Checklist

- Export typed API clients from `app.rs` to be consumed by TUI (PR-0008).
- Add placeholders (no-ops) for `policy`, `memory`, `system_map`, and register them in the app state so PR-0003/0004/0005 can wire without changing API shape.
- Serve `/api/tasks` and `/api/system_map` endpoints returning placeholder data that will be replaced in PR-0004/0005.

Tests

- Unit: config defaults and env override parsing.
- Integration: start server, call `/health`, assert 200 and version string.

Acceptance Criteria

- `cargo run -p assistant-core` starts and logs listening address.
- `/health` responds; graceful shutdown on Ctrl+C.
- All items in Wiring Checklist are present (even if backed by placeholders) and referenced by the follow-up PR IDs.

References

- docs/ARCHITECTURE.md (assistant-core), docs/TESTING.md

Out of Scope

- Policy enforcement, MCP connectivity, memory plane.
