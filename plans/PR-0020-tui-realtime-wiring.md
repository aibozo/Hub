# PR-0020 — TUI Realtime Wiring & Controls

Summary: Wire the TUI to show realtime call status, provide controls (hang up, enable/disable wake), and keep draw code non-blocking. Update help overlay and keymap.

Dependencies: PR-0023 (realtime bridge), PR-0019 (wake sentinel)

## Deliverables
- Status indicator in header (e.g., [🎙 Live] when active).
- Commands: `/voice enable_wake`, `/voice disable_wake`, `/voice status`, `/voice end`.
- Hotkey to hang up (e.g., `Ctrl-Shift-\` or reuse a safe combo), centralized in `keymap.rs`.
- Async calls in `net` module to `/api/realtime/{start,stop,status}`; never block the render loop.
- Toasts and logs on errors; update help overlay (`?`).

## Implementation
- Update `apps/ui-tui/src/keymap.rs` (new hotkey), `apps/ui-tui/src/app.rs` (event handling, status poll), and help text.
- Respect config overrides if `config/tui.toml` exists.
- Add minimal polling/debouncing for status updates.

## Tests
- Snapshot tests of help overlay text and status banner.
- Simulated `/voice` commands update state and call expected `net` routes (mocked).

## Acceptance Criteria
- User can enable/disable wake and end a realtime call from the TUI.
- Status reflects realtime activity; no UI stalls.

## Wiring Checklist
- Key bindings centralized; help overlay updated.
- No I/O in draw; errors surfaced as toasts.
