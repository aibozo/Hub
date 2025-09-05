# PR-0207 — Web UI Final Integration & Wiring Completion

Summary: Polish, wire, and test the Web UI end‑to‑end. Ensure no stubs remain, CORS is correct, `scripts/run-web.sh` works, documentation is complete, and the wiring matrix reflects completion.

Dependencies: PR‑0200..0206.

Deliverables

- Polish pass:
  - Empty states and error toasts across Chat/Agents/Research.
  - Keyboard shortcuts documented in Command Palette footer.
  - Monaco code fences with copy button and language detection fallback.
  - Virtualized lists tuned for performance; code splitting for heavy drawers.
- Scripts:
  - `scripts/run-web.sh` final version: ensure core is up (waiting on `/ready`), then `next dev` (or `next start`).
- Docs:
  - `docs/WEBUI_SPEC.md` finalized; `docs/REPO_LAYOUT.md` and `docs/ARCHITECTURE.md` updated with Web UI.
  - `docs/TESTING.md`: add notes for frontend tests and SSE mocking.
  - Update `docs/WIRING_MATRIX.md` (Web UI: Implemented at PR‑0207).
- CI (optional if present): add web lint/test steps guarded by a `web/` presence check.

Wiring Checklist

- All Web UI features in scope are wired; no placeholder elements remain.
- Running TUI and Web in parallel against the same core works without conflicts.
- Browser can stream via POST‑SSE reliably; keepalive/heartbeat handled if applicable.

Tests

- End‑to‑end smoke: start core + web, create chat, stream a reply, approve a prompt, create an agent and replan, create a task and trigger arXiv brief (feature‑gated/mocked as needed).

Acceptance Criteria

- A developer can run `scripts/run-web.sh`, open the app, chat (with streaming + approvals), manage agents (list/actions/replan), and manage tasks/briefs.
- `docs/WIRING_MATRIX.md` shows Web UI as Completed with no open items.

Out of Scope

- Feature work beyond the spec.

Rollback Plan

- Revert the Web UI series documents and `web/` dir; no data migrations.

References

- docs/WEBUI_SPEC.md
- docs/WIRING_MATRIX.md

