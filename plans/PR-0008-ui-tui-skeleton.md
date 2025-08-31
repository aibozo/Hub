# PR-0008 — TUI Skeleton (ratatui)

Summary: Scaffold the Rust TUI application with basic screens, keymap, and a connection to assistant-core APIs. No tool cards or approvals yet.

Dependencies: PR-0002.

Deliverables

- `apps/ui-tui/` with:
  - `src/main.rs`, `src/app.rs`, `src/theme.rs`, `src/keymap.rs`.
  - `src/screens/{chat.rs,tasks.rs,memory.rs,tools.rs,settings.rs}`.
  - `assets/templates/` for report/brief markdown.
- Keymap: `Ctrl-k` focus input; slash commands stubbed; `/voice` toggles a local flag for now.

Implementation Notes

- Use `ratatui` and `crossterm`; render a statusline with connection/engine state.
- `chat` screen shows a scrollable log (from memory events when wired later).
- `tasks` screen lists tasks from `/api/tasks` (stub endpoint initially).

Wiring Checklist

- Connect to assistant-core endpoints: `/health`, `/api/tasks`, `/api/system_map/digest`.
- Implement `/voice test` command calling core endpoint (PR-0007).
- Add Approvals UI stubs that hit core approvals endpoints (PR-0003), with a simple list/approve/deny flow.
- Tools panel calls core proxy endpoint for a read-only tool (PR-0006).

Tests

- Snapshot test of layout rendering at a fixed terminal size (use `insta` or similar).

Acceptance Criteria

- TUI starts, renders screens, responds to key toggles, and can exit cleanly.
- Can display tasks and system map digest; `/voice test` triggers core endpoint and shows status; approvals list displays real data from core.

References

- docs/ARCHITECTURE.md (TUI), docs/TESTING.md

Out of Scope

- Approvals UI and tool cards (added later).
