# PR-0103 — Core Integration: Manifest, Autostart, Policy, Storage

Summary: Wire the Rust arXiv MCP server into assistant‑core via a manifest, enable autostart, and align policy, storage, and health/status reporting. Replace the Python stub manifest.

Dependencies: PR-0102

Deliverables
- Manifest `config/tools.d/arxiv.json` updated:
  - `transport: "stdio"`, `bin: "./target/release/mcp-arxiv"`, `autostart: true` (keep `FOREMAN_HOME` env passthrough if needed).
- `apps/assistant-core` picks it up and shows status via `/api/tools/status`.
- Policy overlays updated to allow `storage/artifacts/papers/arxiv/**` writes; no broader paths.
- Remove or comment the Python `python -m arxiv_server` line in manifest (document rollback instructions in the plan doc).
- Docs updated for build/run + manifest.

Acceptance Criteria
- `GET /api/tools` lists `arxiv` with tools; `GET /api/tools/status` shows `Connected` after autostart.
- Invocations proxy over stdio and succeed.
- No writes occur outside `storage/`.

Implementation Notes
- Autostart relies on `ToolsManager::autostart()`. Ensure server resolves storage path consistently from env/args.
- Keep in‑core arXiv stubs as fallback only; manifest presence prefers stdio MCP path.
- Provide a small `just arxiv-release` recipe (optional) or document `cargo build -p mcp-arxiv --release` in README.

Tests
- Smoke test: query tool over API → stdio MCP → mocked client returns data.
- Status endpoint shows `Connected` when server is reachable; graceful error on spawn failure.

Wiring Checklist
- Manifest switched to Rust server; autostart enabled.
- Policy overlay allows the precise storage subtree.
- Docs: `docs/TOOLS.md`, `docs/REPO_LAYOUT.md` mention Rust arXiv server.
- WIRING_MATRIX: mark “arXiv MCP — wired to core (stdio/autostart)”. Follow‑ups: 0104–0106.

