# PR-0030 — Agents Schema and Memory Integration

- Owner: core
- Status: Draft
- Depends on: none
- Wires: Memory store, Events/Artifacts linking

## Summary
Add first-class Agent records to the SQLite memory plane with indexes and helpers. Enable linking events and artifacts to a specific agent. This is a pure data + API surface change inside the memory crate; no network.

## Deliverables
- New tables: `Agent`, `AgentIssue`.
- New columns: `Event.agent_id`, `Artifact.agent_id`.
- `crates/foreman-memory` additions:
  - `Agent` and `AgentIssue` types.
  - `create_agent`, `update_agent_status`, `list_agents`, `get_agent`, `append_agent_issue`.
  - `append_event` overload with `agent_id: Option<String>`.
  - `link_artifact_agent(artifact_id, agent_id)`.
- Migrations under `apps/assistant-core/migrations/`.
- Unit tests for CRUD and indexing using in-memory DB.

## Scope
- Schema + Rust methods only.
- No HTTP endpoints or UI.

## Non‑Goals
- Agent runtime loop or orchestration.
- TUI.

## Acceptance Criteria
- Creating, updating, and listing Agents works in tests.
- Events and Artifacts can be linked to an Agent and queried.
- No regressions to existing memory tests.

## Tests
- `memory_agent_crud`: create agent, update status, list.
- `memory_event_linking`: append event with agent_id and retrieve.
- `memory_artifact_linking`: create artifact then link agent_id.

## Wiring Checklist
- Agent data is consumed by API (PR-0031) — OPEN
- Agent events are streamed via API (PR-0036) — OPEN

## Migrations
- Add tables/columns with appropriate indices and FKs; backfill is N/A.

## Rollback Plan
- Schema changes are additive; can be ignored by code if rolled back.

## Risks
- None (additive schema).

