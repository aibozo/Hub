# PR-0035 — Codex MCP Adapter Refinement

- Owner: mcp-servers/rust
- Status: Draft
- Depends on: PR-0033
- Wires: `mcp-codex` bridge robustness and progress capture

## Summary
Harden the Codex stdio MCP adapter: support newline and Content-Length JSON-RPC, capture `session_id` from notifications and results, and record progress/status notifications for the CTR runlog. Keep `autostart` disabled; use `codex-mock` in tests.

## Deliverables
- `mcp-servers/rust/codex` improvements:
  - Tolerant JSON-RPC framing (already mostly implemented).
  - Robust `session_id` extraction from nested payloads and results.
  - Capture progress events; expose in tool response payload for API/runtime consumption.
- `config/tools.d/codex.json`: verify `autostart: false`.
- Unit tests using `codex-mock` to assert session capture and progress collection.

## Scope
- Adapter only; no server-side policy changes.

## Non‑Goals
- Real remote LLM usage (tests must be offline).

## Acceptance Criteria
- `/api/codex/new` and `/api/codex/continue` return session id reliably.
- CTR can read progress for runlog entries.

## Wiring Checklist
- Agents API wraps `/api/codex/*` under an Agent when requested (PR-0036) — OPEN

## Rollback Plan
- Keep existing adapter behavior; no breaking changes.

## Risks
- None; adapter is local and tested with mock.

