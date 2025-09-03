# MCP Tools

Foreman capabilities are provided by MCP servers. Each server is replaceable and adheres to a simple manifest and transport. This document lists the v1 tool set, expected behaviors, and guardrails.

## Core Servers

- mcp-shell: read-only and write modes; supports dry-run, cwd selection, env whitelist. Tools: `exec`, `list_dir`, `read_file`, `write_file` (gated).
- mcp-fs: list/read/write within path policy; directory walkers with size/time limits.
- mcp-proc: list/kill/renice with safe defaults.
- mcp-git: status, branches, worktrees, diff summaries, create/switch branch.

### In-Core Tools (gated)

These are executed inside `assistant-core` and exposed via the same `/api/tools/{server}/{tool}` surface:

- `patch.apply` (server `patch`):
  - Input: `{ "edits": [{ "path": "<file>", "content": "...", "create_dirs": true }] }`
  - Guardrails: no `..` in paths; capped edits per call; approval gate via policy (`apply_patch`).
- `fs.write_text` (server `fs`):
  - Input: `{ "path": "<file>", "content": "...", "create_dirs": true }`
  - For general writes; prefer `patch.apply` for multi-file edits.
- `git.branch|add|commit` (server `git`):
  - Inputs: `{ "path": "<repo root>", "name": "branch" }`, `{ "path": "<repo root>", "patterns": ["."] }`, `{ "path": "<repo root>", "message": "..." }`
  - `commit` is approval-gated by policy.

## Research

- mcp-arxiv: query/date range search; fetch PDFs; cache summaries; “top N of month” with citation proxy; daily brief job.
- mcp-news: curated feeds + dedup + category tags; daily brief; crisis alerts.

### ArXiv MCP (Rust)

- Server: `mcp-arxiv` (stdio), manifest at `config/tools.d/arxiv.json` (autostart enabled).
- Tools (via core API):
  - `POST /api/tools/arxiv/search` with `{ "params": { "query": "...", "max_results": 25, "categories": ["cs.AI"] } }`
  - `POST /api/tools/arxiv/summarize` with `{ "params": { "id": "2501.01234" } }`
  - `POST /api/tools/arxiv/fetch_pdf` with `{ "params": { "id": "2501.01234" } }` (writes under `storage/artifacts/papers/arxiv/` per policy)
  - `POST /api/tools/arxiv/top` with `{ "params": { "month": "2025-01", "n": 5 } }`
- Storage:
  - PDFs under `storage/artifacts/papers/arxiv/<id>/<id>.pdf`
  - Briefs under `storage/briefs/<YYYY-MM-DD>-arxiv.{md,json}`

## Media and Desktop

- mcp-spotify: auth, now playing, queue, playlists.
- mcp-steam: `steam -applaunch <id>`, library list. Optional `config/steamgames.toml` provides a user-maintained `[games]` name→AppID map that is surfaced in the chat system prompt and can be launched via `steam.launch`.
- mcp-emu: wrappers for mGBA, melonDS, PCSX2; per-game profiles; save states.

## System Management

- mcp-installer: apt/snap/flatpak/pip/cargo with plan → explain → dry-run → approve → apply.
- mcp-open: open files/URLs/apps via cross-desktop `xdg-open` abstraction.

## Spec & Debate

- mcp-spec: repo scan → integration plan/spec doc generator; emits architecture + plan files.
- mcp-debate: two-stance orchestrator with judge; outputs report and logs.

## Web

- mcp-websearch: pluggable engines; returns URL + snippet + metadata; robots-aware fetcher/cache.

## Manifests and Transport

- Manifests live in `config/tools.d/*.json` and declare tool names, input schemas, and endpoints.
- Transport is stdio or WS; servers should start with health checks and expose `--dry-run` where relevant.

## Guardrails

- Path policy enforcement, env allowlist, timeouts; no network scans by default; no escalations without core approval.
- Approval gates: `patch.apply` and `git.commit` require an `approval_id`/`approve_token` unless policy returns `Allow`.

## Realtime Exposure

- The realtime V2V bridge publishes a JSON Schema view of available tools to the `gpt-realtime` session.
- Tool requests received from the model are treated identically to regular tool calls: the gatekeeper evaluates policy and collects approvals before execution.
- A synthetic `end_call` tool is available during V2V sessions to terminate the voice call and hand control back to T2T. This tool is non-privileged but audited.
- See `REALTIME.md` and PR-0024 for details on schema generation and bridging.
