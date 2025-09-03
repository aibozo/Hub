# PR-0106 — TUI: Research → arXiv Screen

Summary: Add a dedicated Research → arXiv screen to the terminal UI. Supports new searches, result browsing, PDF downloads, and report preview. Integrates with existing Tools proxy and scheduler trigger.

Dependencies: PR-0105 (reports exist), PR-0008 (TUI skeleton)

Deliverables
- New screen `apps/ui-tui/src/screens/research_arxiv.rs`; register in `screens/mod.rs` and navigation.
- Panels:
  - Left: Saved topics / recent searches / pinned reports
  - Center: Results list from `search_papers` (or recent bundles)
  - Right: Details pane (paper metadata or rendered report preview)
- Keybindings (centralized in `keymap.rs`; documented in `?` help):
  - `n`: New search (prompt for query + categories)
  - `r`: Run daily brief now (`POST /api/schedules/run/arxiv_brief`)
  - `d`: Download PDF for selected paper (policy‑gated)
  - `o`: Open PDF via `proc` (policy‑gated), best‑effort
  - `p`: Pin/unpin report (update artifact pin)
  - `/`: Filter list; `Enter`: Open details; `Tab`/`Shift+Tab`: Cycle panes; `Esc`: Close
- Rendering: Markdown preview for selected report from `storage/briefs/arxiv/...` with theme styling.
- Errors: toasts + log to `storage/logs/ui-tui.log`.

Acceptance Criteria
- Keyboard navigable; help overlay lists bindings accurately.
- Tools status shown for `arxiv`; invoking searches returns results; downloading writes under storage.
- Report preview works and is responsive; no blocking I/O in draw loop.

Implementation Notes
- All HTTP calls centralized in `net` module; debounce polling; non‑blocking UI.
- Focus managed by `App.focus_ix`; reuse shared panels and theme.
- Add an Artifacts tab filter for `type = research_report`.

Tests (where feasible)
- Factor search/filter logic into testable helpers; deterministic behavior for list selection and focus.

Wiring Checklist
- Screen added and registered; keys declared in `keymap.rs` and help overlay updated.
- Net module endpoints used (no direct MCP calls from UI).
- Policy approvals surfaced for downloads/opens.
- Docs: Update `AGENTS.md` (TUI rules reference) and `docs/REPO_LAYOUT.md` (screen added).
- WIRING_MATRIX: mark “TUI research screen — implemented”.

