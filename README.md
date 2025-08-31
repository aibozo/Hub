# Foreman — Local‑First Assistant

Foreman is a local‑first personal assistant with voice and a terminal UI. It orchestrates work via MCP servers (capability providers) and enforces strict policy and approvals, with an efficient memory plane for long‑term recall.

- Agent guide: `AGENTS.md`
- Docs index: `docs/README.md`
- Plans (roadmap): `plans/README.md`

## Features

- Voice and TUI control, with optional TTS streaming server.
- Policy gatekeeper and approvals for all risky actions.
- Memory plane backed by SQLite (tasks, atoms, artifacts, events).
- System Map inventory + compact, pinned digest for prompts.
- MCP tool ecosystem (shell/fs/proc/git, arXiv/news/installer).
- Scheduler for daily briefs and routine jobs.

## Requirements

- Rust: toolchain pinned to `stable` (see `rust-toolchain.toml`). Install `rustfmt` and `clippy`.
- Python 3.11+ for Python MCP servers (optional but recommended).
- OS: Linux/macOS for full experience; Windows supported for TUI + core via PowerShell script.

## Quickstart

Build everything
- `cargo build --workspace`

Run core + TUI together
- Unix/macOS: `bash scripts/run-tui.sh` or `just tui`
- Windows (PowerShell): `scripts/run-tui.ps1`

Run separately (dev)
- Core: `cargo run -p assistant-core`
- TUI: `cargo run -p ui-tui --features tui,http`

The TUI targets `127.0.0.1:6061` by default. The launcher waits for `/ready`, opens the TUI, and stops the core on exit if it started it.

## Configuration

Main config lives in `config/foreman.toml`. Example (default repo values):

```
[foreman]
home = "./storage"
profile = "default"

[voice]
wake_phrase = "hey vim"
stt = { engine = "whisper.cpp", model = "medium" }
tts = { engine = "piper", voice = "en_US-libritts-high" }

[schedules]
arxiv_brief = "07:30"
news_brief  = "08:00"

[mcp]
servers = ["shell", "fs", "proc", "git", "arxiv", "news"]
```

Environment overrides:
- `FOREMAN_CONFIG`: path to an alternate `foreman.toml`.
- `FOREMAN_HOME`: overrides `[foreman].home` at runtime.
- `FOREMAN_PROFILE`: set a profile name for multi‑home setups.
- `FOREMAN_BIND`: core bind address, e.g. `127.0.0.1:6061`.

Policy overlays and tools:
- `config/policy.d/*.yaml`: protect paths, write whitelist, approval keywords, env allowlist, limits, log redactions.
- `config/tools.d/*.json`: MCP manifests (`server`, `tools`, `transport`, `bin`, optional `autostart`).
- `config/schedules.toml`: timezone and `[jobs]` schedule times.

Runtime data lives under `storage/` (sqlite.db, artifacts/, briefs/, logs/, indices/). Never write to repo root during runtime.

## MCP Servers

Manifests in `config/tools.d/*.json` declare available servers. This scaffold includes:
- Rust stdio servers: `mcp-servers/rust/{shell,fs,proc,git}`.
- Python stdio servers: `mcp-servers/python/{arxiv_server,news_server,installer_server}`.
- Voice daemon (optional): `mcp-servers/python/voice_daemon` exposes `/v1/tts/health` and `/v1/tts/stream`.

Build Rust servers
- `just build-servers` or `cargo build -p mcp-shell -p mcp-fs -p mcp-proc -p mcp-git`

Install Rust servers to PATH (optional)
- `just install-servers` or install each with `cargo install --path ...`

Python servers (dev)
- From each package dir: `python -m venv .venv && source .venv/bin/activate && pip install -e .[dev]`
- Run: `python -m <package>` (e.g., `python -m arxiv_server`)

The core autostarts Rust stdio servers marked `"autostart": true` in manifests if `bin` paths are valid.

## API Basics

Core exposes an HTTP/WS API (Axum):
- Health: `GET /health`, `GET /ready`, `GET /metrics`
- Tools: `GET /api/tools`, `GET /api/tools/status`, `POST /api/tools/:server/:tool`
- Policy: `POST /api/policy/check` (preflight decisions)
- Approvals: `GET /api/approvals`, `POST /api/approvals/:id/{approve|deny}`
- System map: `GET /api/system_map`, `GET /api/system_map/digest`, `POST /api/system_map/refresh`
- Memory: search and atoms endpoints
- Scheduler: `GET /api/schedules`, `POST /api/schedules/run/:job`
- Chat: session management and `/control` WS echo (placeholder)

Example: list a temp directory via shell server
```
curl -s http://127.0.0.1:6061/api/tools/shell/list_dir \
  -H 'content-type: application/json' \
  -d '{"params": {"path": "/tmp"}}'
```

## API Cheat Sheet

- Health and Control
  - `GET /health`: Liveness and version.
  - `GET /ready`: Readiness (200 when serving).
  - `GET /metrics`: Prometheus text metrics.
  - `GET /control`: WebSocket echo (dev/testing).

- Voice
  - `GET /api/voice/test`: Placeholder voice test endpoint.

- Tools
  - `GET /api/tools`: List servers and their tools.
  - `GET /api/tools/status`: Connectivity/status per server.
  - `POST /api/tools/:server/:tool`: Invoke a tool. Body: `{ "params": { ... } }`.

- Policy and Approvals
  - `POST /api/policy/check`: Evaluate a proposed action. Body: `{ command, writes, paths, intent? }`.
  - `GET /api/approvals`: List approvals.
  - `POST /api/approvals`: Create approval from proposed action. Body: same as policy check.
  - `POST /api/approvals/:id/approve`: Approve; returns token.
  - `POST /api/approvals/:id/deny`: Deny.
  - `GET /api/explain/:id`: Explain a persisted approval (provenance card).
  - `GET /api/approval/prompt`: Fetch ephemeral approval prompt if present (200 JSON or 204 when none).
  - `POST /api/approval/answer`: Answer ephemeral prompt. Body: `{ id, answer }`.
  - `GET /api/approval/explain/:id`: Explain ephemeral action.

- System Map and Context
  - `GET /api/system_map`: Full `SystemMap` JSON.
  - `GET /api/system_map/digest`: Compact digest string.
  - `POST /api/system_map/refresh`: Trigger a rescan.
  - `POST /api/context/pack`: Build context pack. Body: `{ task_id?, token_budget?, k_cards?, expansions? }`.
  - `POST /api/context/expand`: Expand a handle. Body: `{ handle: "expand://task/<id>|expand://atom/<id>|expand://artifact/<id>#..", depth? }`.

- Memory
  - `GET /api/memory/search?q=...&task_id?=&k?=`: BM25 search; optional task scoping and top‑K.
  - `GET /api/memory/atoms/:id`: Fetch full atom.
  - `POST /api/memory/atoms/:id/pin`: Pin atom.
  - `POST /api/memory/atoms/:id/unpin`: Unpin atom.

- Scheduler
  - `GET /api/schedules`: Snapshot of jobs and next run.
  - `POST /api/schedules/run/:job`: Run a job now (e.g., `arxiv`, `news`).

- Tasks (scaffold)
  - `GET /api/tasks`: List tasks.
  - `POST /api/tasks`: Create task. Body: `{ title, status?, tags? }`.

- Chat
  - `POST /api/chat/complete`: Single reply with tool orchestration; requires `OPENAI_API_KEY`.
  - `POST /api/chat/stream`: SSE stream with events (`token`, `tool_calls`, `tool_call`, `tool_result`, `error`, `done`).
  - `GET /api/chat/sessions`: List chat sessions (files under `storage/chats/`).
  - `POST /api/chat/sessions`: Create a session.
  - `GET /api/chat/sessions/latest`: Latest session info.
  - `GET /api/chat/sessions/:id`: Get session messages.
  - `POST /api/chat/sessions/:id/append`: Append message `{ role, content }`.
  - `DELETE /api/chat/sessions/:id`: Delete session file.

- Misc
  - `GET /api/games`: List available games grouped by console (environment‑specific).

## Repo Layout (current)

- `apps/assistant-core`: Rust orchestrator (HTTP/WS API, gatekeeper, memory, scheduler, tools, system map)
- `apps/ui-tui`: Rust TUI (ratatui) with chat, approvals, tasks, tools, memory, settings
- `crates/`: shared Rust libraries (`foreman-{types,policy,memory,mcp,telemetry}`)
- `mcp-servers/`: Rust and Python MCP servers (`shell,fs,proc,git`, `voice_daemon,arxiv,news,installer`)
- `config/`: `foreman.toml`, `policy.d/`, `tools.d/`, `schedules.toml`
- `storage/`: sqlite, artifacts, briefs, logs, indices (runtime only)
- `docs/`: architecture, policy, memory, system map, tools, testing, wiring matrix
- `plans/`: PR‑by‑PR implementation plans and acceptance criteria
- `scripts/`: launchers and setup scripts (see `scripts/run-tui.sh`)

See `docs/REPO_LAYOUT.md` for the planned full layout and future crates/servers.

## Development

- Install toolchains: `rustup`, Python 3.11, optional `uv`.
- Build: `cargo build --workspace` or `just build`
- Run core: `cargo run -p assistant-core`
- Run TUI: `cargo run -p ui-tui --features tui,http` or `just tui`
- Formatting: `cargo fmt --all`; lint: `cargo clippy --workspace -D warnings`

Conventions:
- Rust: `anyhow`/`thiserror` for errors, `tracing` for logs.
- Python: `ruff`, `black`, type hints, `pytest-asyncio` for async servers.
- Never bypass policy/approvals; all tool calls and subprocesses go through the gatekeeper.

## Testing and CI

Local tests
- Rust: `cargo test --workspace`
- Python: from each server dir: `pytest -q`

Integration tests live under `apps/assistant-core/tests/` (policy, tools, memory, system map, scheduler, metrics).

CI (GitHub Actions)
- Lint: rustfmt, clippy, ruff, black
- Build/test: Rust workspace, Python servers
- See `.github/workflows/{ci.yml,lint.yml}` and `docs/TESTING.md` for details.

## Policy and Safety

All risky actions require explicit approval. The gatekeeper enforces:
- Protect paths, write whitelists, env allowlists, timeouts/limits, redactions (see `docs/POLICY.md`).
- Shell execution is allowlisted by command/args; MCP servers must honor dry‑run where meaningful.

## Troubleshooting

- Can’t connect from TUI: ensure `assistant-core` is bound at `FOREMAN_BIND` (default `127.0.0.1:6061`).
- MCP server status shows errors: verify `config/tools.d/*.json` `bin` paths exist (build servers or adjust paths).
- Inspect logs: `storage/logs/assistant-core-*.log` (launcher) and `storage/logs/ui-tui.log`.
- Increase logging: set `RUST_LOG=debug` before running core/TUI.
- SQLite issues: the core falls back to in‑memory if file‑backed open fails; check permissions under `storage/`.

## Contributing

Start with a plan doc under `plans/PR-XXXX-*.md`, follow `AGENTS.md` for operating rules, and update docs/tests alongside code. Keep changes scoped and deterministic.
