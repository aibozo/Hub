# PR-0031 — Agents API and Supervisor (MVP)

- Owner: core
- Status: Draft
- Depends on: PR-0030
- Wires: HTTP API, supervisor lifecycle

## Summary
Expose CRUD and lifecycle control for Agents over HTTP, and add a lightweight supervisor that tracks Agent status and writes Agent-scoped events. No tool execution yet.

## Deliverables
- API endpoints:
  - `POST /api/agents` → create Agent
  - `GET /api/agents` → list with status filters
  - `GET /api/agents/:id` → snapshot (status, issues, latest events)
  - `POST /api/agents/:id/{pause|resume|abort}`
  - `POST /api/agents/:id/replan` → attach/replace plan artifact (accepts `artifact_id`, or `{ content_md }` to persist under `storage/` and register as Artifact)
  - `GET /api/agents/:id/artifacts` → list linked artifacts
- Supervisor in `apps/assistant-core/src/agents/{mod.rs,runtime.rs}` with in-memory registry and status transitions (`Draft, Running, NeedsAttention, Paused, Blocked, Done, Aborted`).
- Writes Agent events to memory via PR-0030 APIs.
- Unit tests for HTTP handlers (no network I/O).

## Scope
- Wiring to memory store and simple status machine.
- No MCP/tool execution.

## Non‑Goals
- Diff application, validation, or git.
- TUI changes.

## Acceptance Criteria
- Creating, pausing, resuming, aborting Agents via API works and is persisted.
- Agent events show up in memory filtered by `agent_id`.
- OpenAPI-like docs or endpoint schema examples added to `docs/` (brief section).

## Tests
- Handlers return expected shapes and status codes.
- Supervisor changes status and logs events.

## Wiring Checklist
- Tool execution path via gatekeeper (PR-0032/PR-0033) — OPEN
- TUI Agents screen consumption (PR-0034) — OPEN

## Rollback Plan
- Endpoint routes can be disabled; no data loss.

## Risks
- None (no external effects yet).
