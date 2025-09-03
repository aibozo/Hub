# PR-0104 — Research Framework & Context Packer

Summary: Introduce a modular research framework in assistant‑core to orchestrate research tasks across MCP gateways (starting with arXiv). Implement strict context funneling and budgets to prevent oversized prompts and pave the way for multi‑agent expansion.

Dependencies: PR-0103

Deliverables
- New module `apps/assistant-core/src/research/`:
  - `types.rs`: `ResearchTask`, `Topic`, `PaperRef`, `ReportBundle`, `StageResult`.
  - `pipeline.rs`: stages (search → filter → fetch meta/pdf → rank → pack), with budgets and size gates.
  - `pack.rs`: context packing rules with token budgets per stage:
    - Stage budgets (example): search 4k, meta 8k, abstracts 8k, quotes/snippets 6k, final prompt 8k.
    - Global hard cap to avoid exceeding model limits; drop/rollup with heuristics.
- API endpoints (internal or feature‑gated; no network side effects):
  - `POST /api/research/tasks` → create task with query/filters and budgets.
  - `GET /api/research/tasks/:id` → status + results pointer.
- Memory integration: store intermediate `ReportBundle` JSON under `storage/briefs/arxiv/<date>/<slug>.json` and insert an artifact row with `type = "research_bundle"` (hidden) for traceability.
- Deterministic defaults: sort and stable sampling to keep results reproducible.

Acceptance Criteria
- Creating a research task with an arXiv query runs the pipeline using MCP arXiv tools and produces a packed JSON bundle within budgets.
- No model calls in this PR; packing outputs are JSON only.
- Budgets enforce an upper bound on count and total bytes/tokens at each stage; when exceeded, items are trimmed with a clear policy.

Implementation Notes
- Use existing `ToolsManager` to call `arxiv.search_papers`, `arxiv.get_paper`, `arxiv.download_pdf` (optional download gate).
- Ranking heuristic (deterministic): recent first, category match boost, title/abstract BM25‑lite (string scoring), and de‑dup by ID.
- Packer returns normalized `ReportBundle` with compact fields intended for a single synthesis pass in PR‑0105.
- Configurable budgets via `foreman.toml` (`[research.budgets]`) with safe defaults.

Tests
- Unit: budget enforcement, packer trimming behavior, deterministic ordering.
- Integration: end‑to‑end via mocked arXiv MCP responses, producing a fixed `ReportBundle` JSON.

Wiring Checklist
- Research module created and used only by core; no TUI calls yet.
- Context budgets enforced; `storage/briefs/arxiv/**/*.json` written.
- Memory artifact inserted for bundle (type `research_bundle`).
- Docs: add `docs/MEMORY.md` note on artifact type; `docs/ARCHITECTURE.md` research flow diagram.
- WIRING_MATRIX: mark “Research framework (core) — implemented; synthesis pending (0105)”.

