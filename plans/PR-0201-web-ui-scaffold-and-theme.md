# PR-0201 — Web UI Scaffold, Theme, Health

Summary: Scaffold a Next.js (App Router) + TypeScript app under `web/`, add Tailwind + shadcn/ui, wire the dark grey/teal theme tokens, and show a working shell (TopNav/Sidebar) with a live Core health/ready badge.

Dependencies: PR-0200 (CORS), PR-0002 (assistant-core).

Deliverables

- `web/` project with:
  - Next.js + TS configured; Tailwind + shadcn/ui set up.
  - `app/layout.tsx` app shell; `app/page.tsx` default Chat route placeholder.
  - TopNav with tabs (Chat/Agents/Research) and a live status pill using `GET /ready`.
  - Sidebar frame (contextual, empty for now); RightDrawer scaffold with toggle.
  - Theme tokens (CSS variables) for dark grey + teal accent; Tailwind config mapping tokens.
  - `lib/api.ts` with thin fetch helpers and `NEXT_PUBLIC_API_BASE` env.
  - `lib/query.ts` for React Query client; `store.ts` with a minimal UI slice (drawer state).
- `scripts/run-web.sh` to boot the core (if not running) and start Next dev server.
- Docs: `docs/WEBUI_SPEC.md` referenced; `docs/REPO_LAYOUT.md` updated to include `web/` and script.

Wiring Checklist

- Health/Ready badge reflects `/ready` within ~1s polling.
- Tabs are keyboard accessible (arrow keys + focus ring visible).
- No network calls beyond `/ready` in this PR.

Tests

- Unit test for TopNav status pill component (loading/ok/down states).
- Lint/format checks included in package.json scripts.

Acceptance Criteria

- `scripts/run-web.sh` launches a dev server; visiting `/` renders the shell with status pill responding to core health.
- Dark theme tokens apply across shell; accent ring visible on focusable elements.

Out of Scope

- Chat streaming, Agents, Research features (added in subsequent PRs).

Rollback Plan

- Remove `web/` and script; revert docs.

References

- docs/WEBUI_SPEC.md (Sections 2–4, 13)
- docs/REPO_LAYOUT.md

