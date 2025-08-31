# MCP Tools

Foreman capabilities are provided by MCP servers. Each server is replaceable and adheres to a simple manifest and transport. This document lists the v1 tool set, expected behaviors, and guardrails.

## Core Servers

- mcp-shell: read-only and write modes; supports dry-run, cwd selection, env whitelist. Tools: `exec`, `list_dir`, `read_file`, `write_file` (gated).
- mcp-fs: list/read/write within path policy; directory walkers with size/time limits.
- mcp-proc: list/kill/renice with safe defaults.
- mcp-git: status, branches, worktrees, diff summaries, create/switch branch.

## Research

- mcp-arxiv: query/date range search; fetch PDFs; cache summaries; “top N of month” with citation proxy; daily brief job.
- mcp-news: curated feeds + dedup + category tags; daily brief; crisis alerts.

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

## Realtime Exposure

- The realtime V2V bridge publishes a JSON Schema view of available tools to the `gpt-realtime` session.
- Tool requests received from the model are treated identically to regular tool calls: the gatekeeper evaluates policy and collects approvals before execution.
- A synthetic `end_call` tool is available during V2V sessions to terminate the voice call and hand control back to T2T. This tool is non-privileged but audited.
- See `REALTIME.md` and PR-0024 for details on schema generation and bridging.
