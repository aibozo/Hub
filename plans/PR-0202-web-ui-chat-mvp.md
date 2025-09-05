# PR-0202 — Web UI Chat MVP (Sessions, POST‑SSE Streaming)

Summary: Implement a functional Chat tab with sessions CRUD (list/create/get/delete), a streaming reply via `POST /api/chat/stream` using a robust POST‑SSE reader, a sticky Composer, and an Activity drawer for tool events.

Dependencies: PR-0201 (web scaffold), PR-0200 (CORS), Core chat endpoints in place.

Deliverables

- Chat route:
  - SessionList in sidebar: `GET /api/chat/sessions`, create via `POST /api/chat/sessions`, delete via `DELETE /api/chat/sessions/:id`.
  - MessageList in main: render user/assistant bubbles; stream tokens into active assistant bubble.
  - Composer: textarea with Enter/Shift+Enter; Send/Stop/Retry; Agent picker placeholder (wires in PR‑0204).
  - Activity Right Drawer: log `tool_call(s)` and `tool_result` events.
- Streaming:
  - `lib/sse.ts` POST‑SSE reader handling `event:` + `data:` lines with events: `token`, `tool_calls`, `tool_call`, `tool_result`, `error`, `done`.
  - `lib/api.ts` `streamChat()` using `fetch()` with `ReadableStream` and Abort support.
- Monaco code fences (read‑only) for fenced blocks; lazy‑load the editor to keep hydration light.
- Error handling: inline error banner with “Retry” and copyable details; auto‑reconnect on transient failures.

Wiring Checklist

- Stream cancellation via Stop button (AbortController) works without leaks.
- Tool events appear inline (collapsible ToolBlock) and in Activity drawer.
- No blocking network/file I/O in render paths; all via hooks.

Tests

- Mocked SSE stream unit test: `token` accumulation, `done` terminates, and `error` surfaces.
- Session list CRUD tests (mock fetch), ensuring cache updates via React Query.

Acceptance Criteria

- User can create a session, send a message, and receive a streaming assistant reply; tool events logged; Stop/Retry behave as expected.
- Mobile: Composer sticky; MessageList bottom padding prevents overlap; keyboard nav works.

Out of Scope

- Approvals flow (PR‑0203); Agent selection wiring (PR‑0204).

Rollback Plan

- Keep the scaffold; disable Chat components behind a feature flag if necessary.

References

- docs/WEBUI_SPEC.md (Sections 6.1, 9.2, 12 Chat)

