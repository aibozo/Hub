# PR-0034 — Agents TUI (List, Detail, Approvals, Artifacts, Chat)

- Owner: ui-tui
- Status: Draft
- Depends on: PR-0031, PR-0033
- Wires: Agents API to TUI; Approvals drawer

## Summary
Add an Agents screen in the TUI with a left-hand list of agents and a detail view showing plan, runlog, approvals, artifacts, and per-agent chat, adhering to TUI rules (centralized keymap, non-blocking draw, focus management).

## Deliverables
- New module: `apps/ui-tui/src/screens/agents.rs` + registration in `screens/mod.rs`.
- Keymap additions in `apps/ui-tui/src/keymap.rs` for: create agent, pause/resume, open plan, open approvals.
- Net client calls:
  - `GET /api/agents`, `GET /api/agents/:id`, `GET /api/agents/:id/artifacts`
  - `POST /api/agents/:id/{pause|resume}`
  - `POST /api/agents/:id/replan`
  - Approvals: existing `/api/approval/*` endpoints.
- Streaming runlog placeholder via polling (SSE added in PR-0036).
- Toasted errors; logs to `storage/logs/ui-tui.log`.

## Scope
- TUI screens and HTTP client only. No server logic changes.

## Non‑Goals
- SSE (stream) wiring (PR-0036).

## Acceptance Criteria
- Agents screen lists Active/NeedsAttention/Paused/Finished groups.
- Detail view shows latest events, plan markdown, artifacts list, and approvals pane.
- Hotkeys documented in help overlay and function correctly.
- No blocking I/O in draw; UI remains responsive.

## Tests
- Visual/focus logic unit tests where feasible; integration exercised manually.

## Wiring Checklist
- Switch polling → SSE stream (PR-0036) — OPEN

## Rollback Plan
- Hide Agents tab via feature flag or omit from tabs array.

## Risks
- None; read-only operations primarily.

