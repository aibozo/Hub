# PR-0010 — Research Servers (arXiv/News)

Summary: Implement Python MCP servers for arXiv and curated News feeds with caching and summary generation.

Dependencies: PR-0001.

Deliverables

- `mcp-servers/python/arxiv_server/` and `news_server/` packages with `pyproject.toml` and `__main__.py` MCP stdio servers.
- Tools:
  - arXiv: `search(query, date_range)`, `fetch_pdf(id)`, `summarize(id|pdf_path)`, `top(month, n)`.
  - News: `daily_brief(categories)`, `latest(limit)`, `sources()`.
- Artifact writer for cached PDFs and summaries.

Wiring Checklist

- Register manifests in `config/tools.d/{arxiv,news}.json` and ensure assistant-core loads them.
- Provide health checks; core should refuse scheduler job start if servers not ready.
- When summaries are generated, write artifacts and emit memory atoms via core API.

Implementation Notes

- Use robots-aware fetcher with caching; dedup via content hash.
- Summaries: local LLM or heuristic extractive baseline; write markdown artifacts.
- Configurable feeds/categories and citation proxies.

Tests

- Unit: query parsing; cache hit/miss behavior; artifact path construction.
- Integration (optional): mock HTTP and run a brief generation.

Acceptance Criteria

- Servers start and respond to tools; produce artifacts and emit basic telemetry.
- Integration with scheduler (PR-0009) verified with a mocked run producing artifacts and memory atoms.

References

- docs/TOOLS.md, docs/ARCHITECTURE.md (Research), docs/TESTING.md

Out of Scope

- UI rendering of briefs.
