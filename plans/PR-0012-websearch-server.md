# PR-0012 — Websearch Server

Summary: Implement an MCP server for web search with pluggable engines, returning URL + snippet + metadata and a robots-aware fetcher/cache.

Dependencies: PR-0001.

Deliverables

- `mcp-servers/python/websearch_server/` package with tools: `search(query, limit)`, `fetch(url)`, `metadata(url)`.
- Configurable engines (e.g., DuckDuckGo, local index) and backoff.
- Artifact cache for fetched HTML (sanitized) under `storage/artifacts/web/`.

Implementation Notes

- Respect robots and rate limits; sanitize HTML; extract readable text for summaries.

Wiring Checklist

- Register manifest `config/tools.d/websearch.json`; core loads and health-checks it.
- Expose `/api/search` proxy in core to call websearch `search` and deliver minimal results to TUI.
- Wire debate (PR-0013) and spec (PR-0014) servers to call websearch via core proxy.

Tests

- Unit: engine adapter selection; URL sanitization; cache hit/miss.

Acceptance Criteria

- Server responds to `search` with minimal metadata; `fetch` caches content.
- TUI can run a basic `/search` command and display results; debate/spec flows can request search via core.

References

- docs/TOOLS.md, docs/ARCHITECTURE.md (Web)

Out of Scope

- Browser automation or JS-heavy crawling.
