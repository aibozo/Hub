# Testing and CI Strategy

This document defines how to structure and run tests across the Rust workspace and Python MCP servers, plus integration tests that exercise the system end-to-end.

## Test Types

- Unit tests: small, fast, isolated; live adjacent to code (`src/*` with `#[cfg(test)]` for Rust, `tests/` for Python modules).
- Integration tests: validate cross-component behaviors; reside in `apps/assistant-core/tests/` and `tests/` root as needed.
- Golden tests: snapshot outputs for planners, summaries, and reports; store under `tests/golden/`.

## Rust

- Run: `cargo test --workspace`.
- Layout:
  - Crates: unit tests inline, integration tests under `crates/*/tests/`.
  - assistant-core: integration tests under `apps/assistant-core/tests/` (WS API, scheduler jobs, memory packer).
- Guidelines:
  - No network by default; feature-gate networked tests with `--features net-tests`.
  - Use temp dirs (`tempfile`) and ephemeral SQLite files; never write to repo root.
  - Use `tracing` test subscriber to capture logs; assert on key events.
  - Realtime tests: gated under `--features realtime` and use a local mock WS server (`apps/assistant-core/tests/realtime_mock.rs`). No external network is required.

## Python

- Run: `pytest -q` from each server package directory or via a monorepo runner.
- Layout: `mcp-servers/python/*/tests/` with async tests for WS/stdio servers.
- Guidelines:
  - Mark slow/integration tests with `@pytest.mark.integration`.
  - Use `tmp_path` and monkeypatch env; never depend on user home.
  - Mock external services with recorded fixtures or local test servers.

## Integration Tests

- Scope: spin up assistant-core with a minimal config and selected MCP servers in dummy mode.
- Examples:
  - Voice WS playback test: connect to `/v1/tts/stream`, play a 1s tone, assert timely first chunk and graceful stop.
  - Installer dry-run: plan → explain → dry-run for a benign package; assert policy holds without approval.
- Memory pack: create tasks/atoms and verify context pack budgets and expansion ordering.

### Agents Tests

- `apps/assistant-core/tests/agents_memory.rs`: CRUD for `Agent`, event/artefact linking (in-memory DB).
- `apps/assistant-core/tests/agents_api.rs`: API smoke tests (create/list non-panicking).
- `apps/assistant-core/tests/agents_ctr.rs`: CTR runtime behavior:
  - Auto-approval lane completes and writes `CTR_HELLO.txt` under `storage/`.
  - Default lane pauses with an ephemeral approval prompt.
  - Codex planning is best-effort: logs `agent.codex.session` or `agent.codex.unavailable`.

### MCP Adapter Tests

- `mcp-servers/rust/codex/tests/adapter.rs`: exercises the Codex stdio adapter (`mcp-codex`) against `codex-mock` and asserts that `session_id` is captured from notifications. Built in the workspace; runs offline.

## Test Data and Artifacts

- Place durable test fixtures under `tests/fixtures/`.
- Write transient outputs to `target/test-output/` (Rust) or `./.pytest_cache/` (Python defaults); do not commit.

## CI

- GitHub Actions:
  - Lint: `cargo fmt -- --check`, `cargo clippy --workspace -D warnings`, `ruff`/`black --check` for Python.
  - Build & test: `cargo test --workspace --all-features`, `pytest` for each Python package.
  - Caching: Rust `~/.cargo`, `target/`; Python `.venv`/`uv` cache.
  - Artifacts: attach test logs on failure.

## Performance and Health

- Benchmarks (opt-in): voice first-audio latency; memory packer throughput; MCP round-trip latency.

## Frontend (Web UI)

The Web UI is a local Next.js app that consumes `assistant-core` HTTP + POST‑SSE endpoints.

- Start core: `cargo run -p assistant-core` (or `bash scripts/run-web.sh` to start core and web together).
- Start web: `cd web && npm install && npm run dev` (port 3000).
- Configure base: set `NEXT_PUBLIC_API_BASE` in `web/.env.local` when core is not on `127.0.0.1:6061`.

SSE and streaming

- Chat uses POST‑SSE (`POST /api/chat/stream`). The frontend parses events from a `ReadableStream` and supports: `token`, `tool_calls`, `tool_call`, `tool_result`, `error`, `done`.
- Agents events (SSE) stream via `GET /api/agents/:id/events` (emits `status`, `issue`, `approval`, `artifact`, `log`, `ping`).

Unit tests and mocking

- Mock `fetch` and feed a canned SSE byte stream into the `readSSE` async generator for streaming tests.
- Keep tests deterministic; avoid sleeps; debounce inputs as needed.

Accessibility checks

- Verify keyboard traversal: tabs, composer (Enter/Shift+Enter), Command Palette (Cmd/Ctrl‑K), Esc to close overlays.
- Ensure visible focus rings and live region announcements for streaming are present.
