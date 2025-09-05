# PR-0206 — Web UI Settings & Theming

Summary: Add a minimal Settings surface controlling accent color, UI density, font size, code ligatures, and inline tool log preference. Persist to `localStorage` and apply via CSS variables/class toggles.

Dependencies: PR-0201 (theme tokens), prior UI routes.

Deliverables

- Settings UI (page or modal) with 5 controls:
  1) Accent color (select), 2) UI density (Comfortable/Compact), 3) Font size (Small/Default/Large), 4) Code font ligatures (switch), 5) Inline tool logs (switch).
- ThemeProvider: reads persisted settings, applies `data-theme` and CSS vars, and exposes a hook.
- Command Palette (⌘K/Ctrl‑K): search sessions/agents/tasks; quick actions (New chat/agent/task; Toggle Drawer).
- A11y: visible focus rings; SR labels for composer, send/stop, toggles; live region for token streaming.

Wiring Checklist

- Reduced motion respects `prefers-reduced-motion` (disable shimmer; keep text updates instant).
- Settings take effect immediately and persist across reloads.

Tests

- Unit tests for ThemeProvider state transitions and persistence logic.
- Accessibility smoke checks (focus visible) with testing library.

Acceptance Criteria

- Users can adjust theme/density/size/ligatures and see immediate effect; inline tool logs toggle switches between inline ToolBlocks and drawer‑only logs.

Out of Scope

- Server‑persisted user settings; multi‑profile settings.

Rollback Plan

- Remove Settings UI and ThemeProvider; revert to default tokens.

References

- docs/WEBUI_SPEC.md (Settings, Accessibility)

