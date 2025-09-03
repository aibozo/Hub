# PR-0036 — Agents SSE Events and Final Wiring

- Owner: core/ui-tui
- Status: Draft
- Depends on: PR-0030..0035
- Wires: SSE streaming, completes Agents track

## Summary
Add server-sent event streams for per-agent runlogs and wire the TUI to consume them. Close all open wiring checklist items for the Agents track; ensure no stubs remain.

## Deliverables
- API: `GET /api/agents/:id/events` (SSE) with event kinds: `status|issue|approval|artifact|log`.
- Supervisor: fan-out of CTR events to SSE subscribers; backpressure-safe.
- TUI: switch Agents runlog from polling to SSE with reconnect/backoff.
- Docs: update `docs/REPO_LAYOUT.md` and `AGENTS.md` with Agents screen and SSE details.
- Update `docs/WIRING_MATRIX.md` to mark Agents track “wired”.

## Scope
- Streaming and last-mile UI wiring.

## Non‑Goals
- New tools or policy changes (done in PR-0032/0033).

## Acceptance Criteria
- Runlog updates stream live in the TUI.
- Approvals appear and are actionable from Agents screen.
- All items in prior PR Wiring Checklists marked complete or explicitly N/A.

## Tests
- Server SSE smoke test (feature-gated) using a short-lived in-memory agent.
- TUI integration manual verification; deterministic logic unit tests only.

## Wiring Checklist
- Agents API wraps Codex sessions (PR-0035) — CLOSED
- TUI runlog via SSE — CLOSED
- No unlabeled stubs remain for Agents — CLOSED

## Rollback Plan
- Leave polling path as fallback in TUI; keep SSE behind a small feature flag if needed.

## Risks
- SSE resource leakage; mitigate with heartbeat timeouts and drop detection.

