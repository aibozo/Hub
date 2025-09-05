# PR-0205 — Web UI Research (Tasks, Drawer, Brief Triggers)

Summary: Implement the Research tab backed by core Tasks/Memory/Scheduler APIs: a virtualized table of tasks, an editable Task Drawer with tabs for Overview/Artifacts/Atoms/Events, and buttons to trigger arXiv/News briefs.

Dependencies: PR-0201 (web scaffold), PR-0009/PR-0010 (Schedulers + Research servers), memory endpoints.

Deliverables

- Research route:
  - Header with "New Task" (modal), search, and tag filters.
  - Virtualized table (≥500 rows smoothly).
  - Sidebar filters: All / Active / Done + tag chips.
  - Row click opens Task Drawer.
- Task Drawer:
  - Header: editable title; status select; tag editor.
  - Tabs:
    - Overview: description markdown render; due/assignee (optional placeholders, hidden if unused).
    - Artifacts: list with preview (text via Monaco read‑only) + download for binaries.
    - Atoms: list with state and last result.
    - Events: chronological timeline.
- Brief triggers: Buttons to call `POST /api/schedules/run/arxiv` and `/api/schedules/run/news`; Activity drawer shows progress logs.

Wiring Checklist

- No blocking I/O in render; previews load on demand.
- Long lists/table are virtualized; search inputs debounced.
- Brief triggers post and surface progress/errors in Activity drawer.

Tests

- Task create → fetch roundtrip; filter/search behavior.
- Artifact preview for text content uses Monaco; binary shows download affordance.

Acceptance Criteria

- Create a task; list updates; Task Drawer opens and displays artifacts/atoms/events. Brief triggers fire and show activity.

Out of Scope

- Kanban view (optional V2).

Rollback Plan

- Hide Research tab via feature flag; no data migrations needed.

References

- docs/WEBUI_SPEC.md (Research)
- apps/assistant-core/src/api.rs (tasks/memory/schedules)

