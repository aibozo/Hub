# PR-A100 — Quick Win: Python arXiv Server Wiring + Daily Brief

Summary: Ship a working arXiv integration today using the existing Python arXiv MCP stub. Keep scope minimal: ensure manifest is active, implement a basic daily brief using existing stubs, and surface reports in the TUI Reports view. This track is optional if the Rust track is chosen immediately.

Dependencies: PR-0010 (python arxiv stub present), PR-0009 (scheduler base)

Deliverables
- Confirm/adjust manifest `config/tools.d/arxiv.json` to point to `python -m arxiv_server` (already present) and ensure assistant‑core lists tools.
- Scheduler job `arxiv_brief` uses arXiv tools (`search`, `top`, `summarize`, `fetch_pdf`) to materialize a simple Markdown brief (stub content OK for this PR) under `storage/briefs/arxiv/YYYY-MM-DD/*.md`.
- TUI: rely on existing Reports screen to browse briefs; optional tiny Research screen stub that lists recent searches (no multi‑panel yet).

Acceptance Criteria
- `GET /api/tools` lists arXiv; `/api/tools/status` shows reachable (best‑effort — no autostart required).
- Manual trigger `POST /api/schedules/run/arxiv_brief` produces a stub Markdown brief and records an artifact row.
- No external writes beyond `storage/`.

Implementation Notes
- Keep it deterministic; Python server returns stub data already.
- Document the swap path to Rust: replacing only `bin` in `config/tools.d/arxiv.json` post‑PR‑0103.

Tests
- Integration: scheduler writes a brief file and memory artifact using stub data.

Wiring Checklist
- Manifest active and working.
- Scheduler brief implemented.
- Reports visible in TUI Reports screen; optional Research screen stub if trivial.
- WIRING_MATRIX: mark “Quick Win arXiv” as implemented; note replacement by PR‑0103.

