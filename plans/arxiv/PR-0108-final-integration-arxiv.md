# PR-0108 — Final Integration: ArXiv Research Closed Wiring

Summary: Close all wiring for arXiv research across MCP server, core research framework + multiagent scaffold, scheduler brief generation, and TUI research view. Update docs and wiring matrix; add end‑to‑end tests.

Dependencies: PR-0101..0107

Deliverables
- End‑to‑end tests:
  - MCP tool invocation → research pipeline → bundle → (multiagent on/off) → synthesis → artifacts written → TUI lists reports.
  - All network calls mocked; deterministic outputs.
- Docs:
  - `docs/TOOLS.md`: arXiv tool request/response, limits.
  - `docs/ARCHITECTURE.md`: research flow + multiagent diagram.
  - `docs/MEMORY.md`: artifact types `research_bundle`, `research_report`.
  - `docs/WIRING_MATRIX.md`: mark all arXiv rows as wired/complete; no open follow‑ups.
- Config examples:
  - `config/schedules.toml` example topics with `window_days`, `limit`.
  - `config/foreman.toml` `[research]` budgets and agent caps.
- Manifest: `config/tools.d/arxiv.json` final bin path; autostart enabled.

Acceptance Criteria
- CI: `cargo test --workspace` passes; TUI builds with features `--features tui,http`.
- `GET /api/tools/status` shows arXiv Connected by default; manual trigger `POST /api/schedules/run/arxiv_brief` produces artifacts.
- TUI: Research → arXiv shows searches and renders reports; Artifacts tab filters `research_report`.
- No writing outside `storage/`; policy overlays clean and minimal.

Wiring Checklist
- All items in prior PRs checked and closed; any temporary flags removed.
- WIRING_MATRIX updated; PR‑0017 final integration remains green with arXiv items closed.
- No indefinite stubs remain; follow‑ups, if any, link to concrete PR IDs and are not blocking.

