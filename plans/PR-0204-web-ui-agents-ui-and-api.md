# PR-0204 — Web UI Agents (List, Detail, Actions, Use in Chat)

Summary: Implement the Agents tab: list agents, view details with recent events/artifacts, run actions (pause/resume/abort/replan), and start a chat preselected with an agent. Keep edits limited to what core supports today.

Dependencies: PR-0202 (Chat), core Agents API present (create/list/get/actions/artifacts).

Deliverables

- Sidebar `AgentList`: searchable; shows name/status/updatedAt; loads via `GET /api/agents`.
- Detail view:
  - Header: name + model chip + status pill + "Use in Chat" button.
  - Tabs: Overview (summary); Tools (read‑only status via `/api/tools/status` mapping where relevant); Events (recent agent events via `/api/agents/:id/events` SSE); Artifacts (from `/api/agents/:id/artifacts`).
  - Replan: small editor to submit new plan content → `POST /api/agents/:id/replan` with `content_md`.
- Actions: Pause/Resume/Abort buttons wiring to `/api/agents/:id/{pause|resume|abort}`.
- Create Agent: Dialog to create via `POST /api/agents` (fields: task_id, title, root_dir, model?, auto_approval_level?, servers?).
- Chat integration: "Use in Chat" opens a new chat with `agent_id` preselected and passed to Chat stream body.

Notes on Editing

- If a simple update endpoint exists (PUT `/api/agents/:id`), expose limited edits (name/model/auto_approval_level). If not, defer field edits and keep Replan supported.

Wiring Checklist

- Agent SSE/events panel does not block UI; backoff on disconnect.
- Replan writes an artifact and updates row (validated by core tests); Replan events appear in timeline.
- "Use in Chat" passes `agent_id` through `/api/chat/stream` and Chat renders responses in the same session.

Tests

- Agent list fetch + filter; Create flow posts expected JSON.
- Action buttons call correct endpoints; optimistic UI updates with rollback on error.

Acceptance Criteria

- User can create an agent, view details, replan, pause/resume/abort, and start chatting as that agent.
- Artifacts and recent events render correctly; errors surface gracefully.

Out of Scope

- Full agent config editing if core lacks update endpoints (follow‑up PR if desired).

Rollback Plan

- Hide Agents tab via feature flag; leave Chat/Research unaffected.

References

- docs/WEBUI_SPEC.md (Agents)
- apps/assistant-core/tests/agents_api*.rs (behavioral expectations)

