# PR-0101 — Rust arXiv MCP: Scaffold & Tool Schemas

Summary: Create a new Rust MCP stdio server for arXiv with a production‑grade skeleton that fits the repo’s Rust servers, registers tool schemas, and respects storage/policy boundaries. No network calls yet; handlers return structured stubs. Sets the stage for full arXiv integration.

Dependencies: PR-0006 (MCP core servers), PR-0003 (policy), PR-0009 (scheduler base), PR-0010 (existing arXiv stub manifest for compatibility)

Deliverables
- New crate `mcp-servers/rust/mcp-arxiv/` using `rmcp` SDK with stdio bootstrap.
- Tools declared with input schemas: `search_papers`, `get_paper`, `download_pdf`, `build_report`.
- Config flags/env: `--storage-dir`, `FOREMAN_HOME` resolution; default to `storage/artifacts/papers/arxiv`.
- Storage helpers: ensure all writes under `storage/` only (but leave handlers stubbed for now).
- Cargo workspace inclusion; `cargo build --workspace` passes.
- Tests: unit test asserting tool registry/schema shape.

Acceptance Criteria
- `GET /api/tools` shows `arxiv` with four tools when manifest points to this binary.
- `POST /api/tools/arxiv/search_papers` returns a deterministic stub response with correct JSON shape.
- No network access is performed in this PR.
- `storage/` paths only; nothing is written to repo root.

Implementation Notes
- Crate layout:
  - `src/main.rs` — rmcp stdio bootstrap and tool registration.
  - `src/tools/{search,get,download,report}.rs` — handlers; stub data only.
  - `src/config.rs` — parse args/env for storage path, limits.
  - `src/storage.rs` — canonical path builder.
- Manifest (added in PR‑0103) will point `config/tools.d/arxiv.json` to `./target/debug/mcp-arxiv` initially.

Tests
- Unit: tool schema JSON matches expected fields; name parity with manifest.
- Integration (non‑net): spawn server process, send one request over stdio, assert stub response.

Wiring Checklist
- MCP transport: stdio only; tools registered with valid JSON Schemas.
- Storage: only under `storage/artifacts/papers/arxiv`.
- Policy: no new writes yet; no changes required in this PR.
- Docs: add crate to `docs/TOOLS.md` (overview paragraph) and `docs/REPO_LAYOUT.md` (servers list).
- WIRING_MATRIX: add row “arXiv MCP (Rust)” = scaffolded, follow‑up PRs: 0102–0103.

