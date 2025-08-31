# Foreman — Agent Operating Guide

Use this guide as your pinned context when contributing. It is dense by design: follow it to avoid scope sprawl, keep tests clean, and align with the architecture.

## Project Overview

- Local-first personal assistant with voice + TUI control.
- Capabilities provided by MCP servers; core orchestrator in Rust enforces policy and memory hygiene.
- Reference docs: `docs/ARCHITECTURE.md`, `docs/POLICY.md`, `docs/MEMORY.md`, `docs/SYSTEM_MAP.md`, `docs/TOOLS.md`, `docs/REPO_LAYOUT.md`, `docs/TESTING.md`.

## Principles

- Small, reviewable PRs; tight scopes aligned to a single plan doc under `plans/PR-XXXX-*.md`.
- Never bypass policy/approvals. All tool calls and subprocesses go through the gatekeeper.
- Deterministic tests; no network or external state unless explicitly feature-gated and mocked.
- Don’t refactor unrelated code in feature PRs. If needed, open a dedicated cleanup PR.

## Wiring Policy

- No indefinite stubs: if a stub is introduced, the same PR must either wire it fully or explicitly link to the exact follow-up PR by ID and add an item in `docs/WIRING_MATRIX.md`.
- Every PR must include a “Wiring Checklist” and update it during review until all items are satisfied or linked to a follow-up PR.
- Final integration occurs in PR-0017; that PR cannot merge if any wiring checklist item remains open.

## Workspace and Boundaries

- Rust workspace for core/TUI/crates. Python workspace for voice + research MCP servers.
- Keep crates narrowly scoped with clear seams (types/config/policy/memory/mcp/exec/system-map/telemetry).
- Store artifacts and transient data under `storage/`; NEVER write into repo root during runtime.

## Build

- Rust: `cargo build --workspace`.
- Python (per server):
  - Create venv (uv or venv): `uv venv && source .venv/bin/activate && uv pip install -e .[dev]`.
  - Or `python -m venv .venv && source .venv/bin/activate && pip install -e .[dev]`.
- Optional task runner: if `Justfile` is present, use `just build`.

## Run (Dev)

- Core: `cargo run -p assistant-core`.
- TUI: `cargo run -p ui-tui`.
- Voice daemon: `python -m voice_daemon` (ensure models are set up per `docs/ARCHITECTURE.md` TTS section).
- MCP servers: start from their package dirs (`python -m arxiv_server`, etc.) or via a dev script when available.

## Tests

- Rust: `cargo test --workspace`.
- Python: from each server package: `pytest -q`.
- Integration:
  - `apps/assistant-core/tests/` contains WS API, scheduler, memory packing tests.
  - Feature-gate networked tests with `--features net-tests` (Rust) or `@pytest.mark.integration` (Python).
- See `docs/TESTING.md` for structure, fixtures, and CI matrix.

## PR Expectations

- Start from a corresponding plan under `plans/` and follow its Deliverables and Acceptance Criteria.
- Include unit tests for new logic and update relevant integration tests.
- Update docs when changing interfaces or behaviors.
- Keep commit messages conventional: `feat(scope): summary`, `fix(scope): ...`, `docs(scope): ...`, `test(scope): ...`.

## Policy and Safety

- Path policies, env allowlists, timeouts, and dry-run behavior are mandatory. See `docs/POLICY.md`.
- Risky actions require an approval token and must surface an Explain-This provenance card.

## File/Code Conventions

- Rust: `rustfmt`, `clippy -D warnings`, error handling via `anyhow`/`thiserror`, `tracing` for logs.
- Python: `ruff`, `black`, `pytest-asyncio` for async servers, type hints (mypy-friendly).
- Config: TOML for `foreman.toml`; YAML overlays for policy; JSON manifests for MCP tools.

## TUI Implementation Rules

- Modular screens: add each major view under `apps/ui-tui/src/screens/<name>.rs` and register it in `screens/mod.rs`. Keep draw logic and per‑screen state local to that module.
- Centralized hotkeys: declare all bindings in `apps/ui-tui/src/keymap.rs` (no ad‑hoc key handling scattered across screens). Update the help overlay whenever keys change. If `config/tui.toml` exists, wire new actions to its overrides.
- Navigable by keyboard: every interactive element must be reachable via common keys: arrows or `h/j/k/l` to move, `Tab`/`Shift+Tab` to cycle focus, `Enter` to activate, `Esc` to cancel/close, `?` to toggle help. Provide visible focus/selection highlights.
- Selectable widgets: lists/tables/grids intended for selection must use stateful widgets with a `selected` index and scrolling; do not implement “click‑only” affordances. Space toggles checkable items when relevant.
- Focus management: maintain a single source of truth for focus (e.g., `App.focus_ix`). Do not let individual widgets self‑manage global focus.
- Non‑blocking UI: never perform network/filesystem I/O in draw code. Use async tasks in a `net`/client helper, update state via messages, and debounce polling. The UI loop should render within budget and remain responsive.
- Error handling: surface user‑visible errors as toasts and log details to `storage/logs/ui-tui.log`. Do not panic on recoverable errors.
- Consistent layout: reuse common panels (headers/footers/status bars) and styling from `theme.rs`; avoid one‑off styles without a compelling reason.
- API access: centralize HTTP endpoints and payload shapes in a single `net` module; reuse across screens to avoid drift.
- Help overlay: every screen must document its key bindings and primary actions in the built‑in help (`?`). Keep it accurate as bindings evolve.
- Mouse optional: support mouse interactions where natural, but keyboard UX remains primary; never require the mouse to access functionality.
- State boundaries: keep cross‑screen coupling minimal. Share only through top‑level `App` state or well‑scoped helpers; avoid hidden globals.
- New components: when adding a major TUI screen/flow, update `AGENTS.md` (Codebase Map + TUI rules if needed) and `docs/REPO_LAYOUT.md` as appropriate.
- Tests and determinism: where feasible, factor logic to testable helpers and keep deterministic timing (no sleep‑loops). Avoid network calls in tests.
- Accessibility/readability: maintain sufficient color contrast; prefer concise labels; avoid truncating essential information without an expansion path.

## Where to Add Things

- New MCP server (Rust): under `mcp-servers/rust/<name>/` with its manifest in `config/tools.d/<name>.json`.
- New MCP server (Python): under `mcp-servers/python/<name>_server/` with `pyproject.toml` and tests.
- Core feature: `apps/assistant-core/src/<area>/`; shared logic as a crate under `crates/`.
- Docs: extend `docs/` and reference from your PR plan.

## Don’t Forget

- Wire new tools into `config/tools.d/` and `config/foreman.toml`.
- Add migrations (`apps/assistant-core/migrations/`) if schema changes.
- Ensure CI passes locally (lint/build/test) before submitting.

## Codebase Map

- Root workspace: Rust + Python mono-repo. See `docs/REPO_LAYOUT.md` for the planned structure and `Cargo.toml` for current members.
- `apps/assistant-core` (Rust): orchestrator daemon and HTTP/WS API.
  - `src/main.rs`: boot, config load, Axum server.
  - `src/api.rs`: API routes (health, tasks, memory, tools proxy, approvals, chat sessions, schedules, policy).
  - `src/app.rs`: service wiring (policy, approvals, provenance, memory, system map, tools, scheduler).
  - `src/gatekeeper/*`: policy engine wrapper, approvals store, provenance cards.
  - `src/memory/*`: Memory wrapper over `crates/foreman-memory`; context packer.
  - `src/system_map/*`: scanner, model, digest, persistence and eventing.
  - `src/tools.rs`: MCP tool loader, stdio transport, in-core stubs, shell allowlist.
  - `src/scheduler.rs`: cron-like jobs for briefs; persists artifacts/events.
  - `src/config.rs`: `foreman.toml` parsing and env overrides.
  - `src/metrics.rs`, `src/telemetry.rs`: counters and tracing setup.
  - `migrations/`: SQLite migrations for memory store.
  - `tests/`: integration tests covering health, policy, tools, memory, system map, schedules, installer paths.
- `apps/ui-tui` (Rust): terminal UI (ratatui).
  - `src/main.rs`: feature-gated entry (`tui`,`http`).
  - `src/app.rs`: App state, event loop, fetches health/digest/tasks/approvals.
  - `src/screens/*`: Chat, Dashboard, Tasks, Memory, Tools, Reports, Settings.
  - `src/keymap.rs`, `src/theme.rs`: input and styling.
- `crates/` (Rust libs): shared seams.
  - `foreman-types`: basic shared types (e.g., `VersionInfo`).
  - `foreman-policy`: YAML policy rules, loader/merge, and evaluator.
  - `foreman-memory`: async SQLite store (Tasks/Atoms/Artifacts/Events) via `sqlx`.
  - `foreman-mcp`: request/response types for stdio MCP.
  - `foreman-telemetry`: basic telemetry scaffold.
- `mcp-servers/`:
  - Rust: `shell`, `fs`, `proc`, `git` (stdio servers matching manifests under `config/tools.d/*.json`).
  - Python: `voice_daemon` (WS TTS health/stream), `arxiv_server`, `news_server`, `installer_server` (stdio tools).
- `config/`:
  - `foreman.toml`: home, voice, schedules, MCP server list.
  - `policy.d/*.yaml`: safety overlays (protect paths, write whitelist, approval keywords, env allowlist, limits, redactions).
  - `tools.d/*.json`: MCP manifests (server, tools, transport, bin, autostart).
  - `schedules.toml`: timezone and `jobs` table (e.g., `arxiv`, `news`).
- `storage/`: runtime data (sqlite.db, artifacts/, briefs/, logs/, indices/). Never write to repo root.
- `docs/`: architecture/policy/memory/system-map/tools/testing and wiring matrix.
- `plans/`: PR-by-PR plan docs and acceptance criteria.
- `scripts/`: `run-tui.sh` launcher, setup scripts.

## Subsystems

- Assistant Core: `apps/assistant-core`
  - API endpoints (from `src/api.rs`):
    - `/health`, `/ready`, `/metrics`, `/control` (WS)
    - `/api/tasks` (GET/POST)
    - `/api/system_map`, `/api/system_map/digest`, `/api/system_map/refresh`
    - `/api/context/pack`, `/api/context/expand`
    - `/api/memory/search`, `/api/memory/atoms/:id` (+ `/pin` `/unpin`)
    - `/api/schedules`, `/api/schedules/run/:job`
    - `/api/tools`, `/api/tools/status`, `/api/tools/:server/:tool`
    - `/api/approval/prompt`, `/api/approval/answer`, `/api/approval/explain/:id`
    - `/api/chat/*` (sessions list/create/get/delete/append, complete/stream)
    - `/api/policy/check`, `/api/approvals*`, `/api/explain/:id`
  - Gatekeeper: wraps `crates/foreman-policy` with `ProposedAction -> PolicyDecision`; tracks approvals and provenance.
  - Memory: initializes file-backed SQLite under `storage/sqlite.db` with fallback to in-memory; emits Events on map changes and scheduler runs.
  - System Map: lightweight scanner with strict timeouts; persists `storage/map.json`; produces a compact digest for prompts.
  - Scheduler: computes next run from `schedules.toml`; writes briefs under `storage/briefs/` and logs memory events/artifacts.
  - Tools Manager: loads manifests, autostarts stdio servers, proxies tool calls; enforces `shell.exec` allowlist.
- UI TUI: `apps/ui-tui`
  - Screens: Chat, Dashboard (health/metrics/schedules/reports), Tasks, Memory (search/card view/pack summary), Tools (invoke + params editor), Reports, Settings.
  - Requires features: run with `--features tui,http` to enable HTTP calls to core.
- MCP Servers
  - Rust stdio servers: `mcp-servers/rust/{shell,fs,proc,git}` with matching tools and policy enforcement.
  - Python stdio servers: `mcp-servers/python/{arxiv_server,news_server,installer_server}` (stubbed but wired via manifests).
  - Voice daemon (Python): `mcp-servers/python/voice_daemon` exposes `/v1/tts/health` and `/v1/tts/stream` (WS or HTTP-chunked fallback).

## Docs Index

- `docs/ARCHITECTURE.md`: system boundaries, flows, contracts, observability, config.
- `docs/POLICY.md`: approvals model, classes, overlays, enforcement points.
- `docs/MEMORY.md`: stores, context packer, schema, indexing, privacy.
- `docs/SYSTEM_MAP.md`: inventory scope, digest rules, scanning strategy.
- `docs/TOOLS.md`: server catalog, guardrails, manifests/transport.
- `docs/TESTING.md`: test types, structure, CI matrix.
- `docs/WIRING_MATRIX.md`: what is implemented vs wired; required for stubs.

## Build, Run, Test

- Build: `cargo build --workspace` or `just build`.
- Run core: `cargo run -p assistant-core`.
- Run TUI: `cargo run -p ui-tui --features tui,http` or `just tui`.
- Python servers: within each package, `python -m <pkg>` after editable install; the core autostarts Rust stdio servers if `bin` is configured.
- Tests:
  - Rust: `cargo test --workspace` (integration tests under `apps/assistant-core/tests/`).
  - Python: `pytest -q` per package; see `Justfile` and `docs/TESTING.md`.

## Config and Policy

- Main config: `config/foreman.toml` (home/profile, voice engines, schedules, MCP servers list).
- Policy overlays: `config/policy.d/*.yaml` merged lexicographically; enforce protect paths, write whitelist, approvals, env allowlist, limits, redactions.
- MCP manifests: `config/tools.d/*.json` with `server`, `tools`, `transport` (`stdio`), `bin`, optional `autostart`.
- Schedules: `config/schedules.toml` with `timezone` and `[jobs]` times.

## Densities and Hotspots

- Code hotspots (files, approx lines):
  - `apps/assistant-core`: 33 files, ~2.9k lines (core logic, API, tools, scheduler, system map).
  - `apps/ui-tui`: 13 files, ~1.7k lines (screens and UI loop).
  - `crates/foreman-memory`: ~400 lines (SQLite store and queries).
  - `crates/foreman-policy`: ~140 lines (policy rules/eval).
  - MCP Rust servers combined: ~400 lines (stdio handlers and tools).
  - Python servers combined: ~450 lines (voice + research + installer stubs).
- Tests: `apps/assistant-core/tests/*` cover policy, tools, memory, system map, schedules, metrics, installer.

## Common Tasks

- Add a tool/server:
  - Implement under `mcp-servers/{rust,python}/<name>` and expose stdio handler.
  - Add manifest in `config/tools.d/<name>.json` and include in `mcp.servers` in `foreman.toml`.
  - If writes or risky actions: update `config/policy.d/*` and ensure approval flow is enforced.
- Extend memory/API:
  - Schema changes → add SQL in `apps/assistant-core/migrations/` and update `crates/foreman-memory` methods.
  - Expose endpoints in `apps/assistant-core/src/api.rs` and add tests under `apps/assistant-core/tests/`.
- Update scheduler:
  - Edit `config/schedules.toml`; wire job in `apps/assistant-core/src/scheduler.rs` and surface artifacts/events.

## Gotchas

- Never bypass the gatekeeper: all subprocess/MCP calls must go through policy checks and approvals.
- Keep runtime writes under `storage/`; integration tests should use temp dirs or in-memory DBs.
- Prefer stdio MCP with explicit manifests; TUI expects core at `127.0.0.1:6061` by default.

## Local Launchers

- Emulators: See system prompt section in `apps/assistant-core/src/api.rs` for DS and GB/GBA usage. Only the allowlisted commands are permitted.
- Steam: Quick‑launch mapping lives in `config/steamgames.toml` and is injected into the chat system prompt.
  - File format:
    - `[games]` table mapping display names to AppIDs. Example:
      - `Overwatch 2 = 2357570`
      - `No mans sky = 275850`
      - `Peak = 3527290`
      - `Cyberpunk = 1091500`
      - `Dishonored 2 = 403640`
  - Launch policy:
    - Preferred: call `steam.launch` with `{ "appid": "<APPID>" }`.
    - Or via `shell.exec` with `cmd: "steam", args: ["-applaunch", "<APPID>"]` (whitelisted).
    - Do not use `steam://rungameid/<APPID>` unless allowlisted explicitly.
