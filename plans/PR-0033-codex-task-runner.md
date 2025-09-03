# PR-0033 — Codex Task Runner (CTR) Orchestration Loop

- Owner: core/agents
- Status: Draft
- Depends on: PR-0031, PR-0032
- Wires: Agents runtime to ToolsManager, Policy, Memory

## Summary
Implement the agent runtime loop (Plan → Implement → Validate → Commit → Report) that executes steps via gated in‑core tools and Codex MCP for planning. Tests use the `codex-mock` server; no network.

## Deliverables
- `apps/assistant-core/src/agents/runtime.rs` state machine with steps:
  - `ProposeDiff` (Codex MCP), `ApplyDiff` (patch.apply), `Validate` (shell.exec/proc.list as needed), `Commit` (git.*), `Report` (artifacts/events).
- Policy pre‑flight for every step using `PolicyEngine::evaluate` and ephemeral approvals for Warn/Hold.
- Context pack on start via existing `/api/context/pack` logic (call internal helper).
- Artifacts: diff bundles and validation logs stored and linked to `agent_id`.
- Unit tests covering: all‑green run, approval required path, validation failure → repairs up to N attempts.
- Config: extend `config/foreman.toml` with `[agents]` defaults (`default_model`, `auto_approval_level`, `max_repair_attempts`).

## Scope
- Runtime + in‑process orchestration only.

## Non‑Goals
- SSE streaming (added in PR-0036).
- TUI.

## Acceptance Criteria
- A created Agent can be started via supervisor; CTR runs and updates status accordingly.
- All writes occur through in‑core tools; no direct FS writes outside tools.
- Memory reflects events and artifacts per step.

## Tests
- `ctr_runs_all_green`.
- `ctr_escalates_then_resumes_after_approval`.
- `ctr_stops_after_max_repairs`.

## Wiring Checklist
- Agents TUI consumes CTR status/events (PR-0034) — OPEN
- SSE event stream (PR-0036) — OPEN

## Rollback Plan
- Disable CTR start/loop behind feature flag if needed.

## Risks
- Long‑running loops; ensure timeouts and deterministic tests with mocks.
