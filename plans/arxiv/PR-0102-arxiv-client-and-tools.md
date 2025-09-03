# PR-0102 — ArXiv Client & Tools (search/get/download)

Summary: Implement arXiv API client and wire tool handlers to real data with strict limits, timeouts, and deterministic tests via mocked responses. Parse Atom feed, normalize fields, and support polite pagination.

Dependencies: PR-0101

Deliverables
- `arxiv::client` using `reqwest` (rustls), `quick-xml` to parse Atom feed from `https://export.arxiv.org/api/query`.
- Tool handlers:
  - `search_papers(query, categories[], from, max_results<=50, sort_by)` → list of paper cards (id, title, authors[], primary_category, updated, links: {html,pdf}, summary).
  - `get_paper(id)` → canonical single metadata record.
  - `download_pdf(id)` → download to `storage/artifacts/papers/arxiv/<id>/<id>.pdf` and write `meta.json`.
- Limits & behavior:
  - Timeout 15s; 2 retries; rate limit ≤3 req/s (sleep).
  - Pagination support via `start`, `max_results`; clamp to 200 total per call.
  - Path safety checks; only write under `storage/`.
- Tests:
  - Unit: XML sample parsing; query builder; path builder.
  - Integration: HTTP mocked fixtures; verify shape and storage writes (no live network).

Acceptance Criteria
- `POST /api/tools/arxiv/search_papers` returns realistic, normalized records from mocked fixtures.
- `download_pdf` writes files under the expected directory and returns the absolute path.
- All tests pass offline; no external network used in CI.

Implementation Notes
- Types: `PaperCard`, `PaperMeta`, `PaperLink` structs with `serde`.
- Respect categories by tokenizing category filters (primary + additional categories).
- Derive a stable `id` from feed `id` (e.g., `2501.01234`), normalizing legacy `arXiv:NNNN` forms.

Wiring Checklist
- Tools: three handlers implemented with real client.
- Storage: PDF+meta placed under `storage/artifacts/papers/arxiv/<id>/`.
- Policy: no additional changes; storage path remains whitelisted scope.
- Docs: update `docs/TOOLS.md` with request/response examples; `docs/TESTING.md` with arXiv fixtures strategy.
- WIRING_MATRIX: mark “arXiv MCP (Rust)” = tools implemented (search/get/download). Follow‑up PRs: 0103–0105.

