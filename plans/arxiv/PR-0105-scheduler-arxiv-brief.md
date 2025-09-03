# PR-0105 — Scheduler Job: arxiv_brief (Reports + Artifacts)

Summary: Add `arxiv_brief` scheduler job that runs daily topics, executes the research pipeline, synthesizes a Markdown report from the packed bundle using the core chat completion endpoint, and stores artifacts in `storage/briefs/arxiv/` with memory entries.

Dependencies: PR-0104

Deliverables
- Scheduler integration:
  - Read topics from `config/schedules.toml` under `[jobs.arxiv_brief.topics]`.
  - For each topic: run research pipeline → produce `ReportBundle` → synthesize to Markdown using a dedicated “research‑brief” system prompt via `/api/chat/complete` (or internal helper if already exposed).
- Artifacts written per topic:
  - Markdown: `storage/briefs/arxiv/YYYY-MM-DD/<slug>.md`
  - JSON: `storage/briefs/arxiv/YYYY-MM-DD/<slug>.json` (bundle)
  - Memory: insert artifact with `type = "research_report"` and `meta` digest (query, window, paper ids, pinned=false).
- API Support: `POST /api/schedules/run/arxiv_brief` triggers the job immediately.
- Prompt: add a stock prompt template under core (deterministic; no network tools during synthesis).

Acceptance Criteria
- Manual trigger produces Markdown and JSON for each configured topic with stable formatting and section headers.
- Memory artifact entries created and retrievable via existing memory search endpoints.
- Runs within configured budget; number of papers and summary length bounded.

Implementation Notes
- Chat call uses the packed JSON as context (not raw abstracts) to keep token usage bounded.
- Optionally download PDFs for top‑K items (configurable) but do not feed PDFs into model in this PR.
- Include a report header with topic, date, total papers considered, top picks, and references.

Tests
- Integration (mocked arXiv and deterministic chat stub): scheduler run writes files; artifact row inserted; filenames match expected.

Wiring Checklist
- Scheduler reads topics; job registering under `apps/assistant-core/src/scheduler.rs`.
- Report prompt stored in code with a stable template.
- Artifacts and memory entries created.
- Docs: `docs/TOOLS.md` (usage via `/api/schedules/run/arxiv_brief`), `docs/REPO_LAYOUT.md` (briefs dir), `docs/TESTING.md` (mocked scheduler test).
- WIRING_MATRIX: mark “arxiv_brief — implemented (reports + artifacts)”.

