# Foreman Architecture Reference

This document captures the core architecture for the Personal Assistant “Foreman”. It defines system boundaries, responsibilities, interfaces, and invariants. All PR plan docs should reference this file for shared contracts and vocabulary.

## Goals and Scope

- Always-on, local-first assistant with voice and TUI control.
- Modular capability surface via MCP servers, hardened by a policy layer and explicit approvals.
- Hierarchical memory with strong recall without destructive compaction.
- Research/reporting agents and daily schedulers (arXiv/news briefs).

Non-goals for v1: full GUI/mobile app, unconstrained desktop control, cloud-only dependencies.

## Top-Level Processes

1. assistant-core (Rust)
   - Orchestrator, policy gatekeeper, task lifecycle, memory plane, schedulers, MCP client, WS/HTTP control API.
2. voice-daemon (Python)
   - Wake word, VAD, STT (whisper.cpp/faster-whisper), TTS (CosyVoice2/OpenVoice/Kokoro/Piper) with streaming.
3. mcp-servers/*
   - Capability providers: shell/fs/proc/git, research (arXiv/news/websearch), media/desktop (spotify/steam/emu), installer, debate, spec.
4. ui-tui (Rust/ratatui)
   - Terminal UI: chat, approvals, tasks, memory search, tools, settings. Can toggle voice.
5. storage
   - SQLite for log/tasks/configs, Tantivy (BM25), HNSW/FAISS for embeddings, object store for artifacts.
6. mobile-bridge (PWA)
   - Optional WS client + push notifications (later phase).

## Data Flows

- Voice: mic → VAD → STT → assistant-core → plan → gated tool calls (MCP) → results → memory update → TTS stream → audio output.
- TUI: command → assistant-core → same flow. Approvals and Explain-This inlined as cards.
- Schedulers: cron-like jobs trigger arXiv/news agents → cache summaries/artifacts → morning brief.
  - Research pipeline: `arxiv.search` → filter/rank → pack bundle (budgets) → optional multiagent selection (workers+judge) → synthesize Markdown brief.

## Component Contracts

### assistant-core (Rust)

- Must expose:
  - WS/HTTP API for TUI/mobile: task list, approvals, memory search, voice control, metrics.
  - MCP client with transport (stdio or WS) and tool registry from `config/tools.d/*`.
  - Policy gatekeeper that classifies actions (safe/warn/block) and enforces approvals and dry-runs.
- Memory plane APIs: append events, write atoms, query/search (BM25 + vector), build context packs under token budget.
- Research pipeline: bounded context packer enforcing per-stage budgets; multiagent selection is opt-in and uses strict token limits per worker.
- Realtime V2V Bridge: feature-gated WS/WebRTC client that configures a `gpt-realtime` session, exposes core tools as JSON Schemas, and mediates tool-calls via the gatekeeper. Provides `/api/realtime/{start,stop,status}` and integrates with wake sentinel and TUI controls. See `REALTIME.md`.
  - Scheduler with timezone-aware cron expressions; task creation hooks; artifact URIs (`artifact://...`).
- Concurrency model: tokio runtime, bounded channels for backpressure, per-task spans for tracing.
- Configuration: TOML (`config/foreman.toml`) + YAML policy overlays (`config/policy.d/*.yaml`).

### voice-daemon (Python)

- WS endpoint `/v1/tts/stream` for streaming PCM (24kHz mono) with first-audio latency metrics and barge-in support.
- Optional HTTP health/bench endpoints.
- Engine registry with capability flags (streaming, clone, markers, license) and fallback logic.
- Wake word + VAD + STT pipeline with push-to-talk or wake word trigger.
  - Note: Wake sentinel integration with the core realtime bridge is detailed in `REALTIME.md` and PR-0019.

### mcp-servers

- Each server has a manifest under `config/tools.d/` and a minimal set of tools with clear inputs/outputs.
- Transport: stdio or WebSocket. Servers must honor `--dry-run` where meaningful and surface plan/explain/apply.
- Security: adhere to path policies and environment allowlists; never escalate without explicit approval from core.

### ui-tui (Rust)

- Connects to assistant-core, renders chat/task/memory/tools/settings.
- Hotkeys and slash-commands (e.g., `/voice`, `/install`, `/spec <dir>`), plus inline approval prompts.
- Barge-in stop for TTS playback when space is pressed or wake word detected.

### Agents (CTR)

- Agent runtime inside `assistant-core/src/agents/*` manages long-running feature tasks with a resumable loop: Plan → Apply → Validate → Commit → Report.
- Planning uses Codex MCP (best-effort) for diffs; all mutations flow through in-core gated tools (`patch.apply`, `git.*`).
- Policy preflight before every step; “Warn/Hold” triggers an ephemeral approval prompt (and optional persisted approval token) surfaced in the TUI.
- Events and artifacts are recorded with `agent_id` backrefs for runlogs and traceability.
- HTTP API: `/api/agents{,/:id,/pause,/resume,/abort,/replan,/artifacts}`; a TUI “Agents” tab shows list and per-agent runlog.

## Storage and Data Model

- SQLite tables: Task, TaskDigest, Atom, Artifact, Event (append-only event log).
- Indices: Tantivy (BM25) and HNSW/FAISS for embeddings; namespaces per global/task/spec.
- Objects: artifacts in `storage/artifacts/`; quarantined downloads with checksums in `storage/quarantine/`.

## Policy and Approvals

- Command classes: Safe (read/list/open), Warn (writes in home, package installs), Block-by-default (sudo, system changes, deletes outside whitelist, network changes).
- Approval flow: plan card → policy eval → hold or allow → user approval/deny → supervised execution (pty wrapper, timeouts, logging).
- Provenance: Explain-This displays sources, hashes, vendor/package origins, and dry-run results.

## Observability

- Structured logging via `tracing` with per-task spans. Prometheus metrics and health endpoints from core.
- voice-daemon reports `t_first_ms`, RTF, and engine readiness.

## Configuration Files

- `config/foreman.toml`: top-level settings (home/profile, voice, schedules, tool list, TTS preferences).
- `config/policy.d/*.yaml`: layered safety policy (defaults + local overrides).
- `config/tools.d/*.json`: MCP server manifests with tool definitions and endpoints.
- `config/schedules.toml`, `config/tui.toml`: cron jobs, theme/keymap.

## Build and Workspaces

- Rust workspace for core, TUI, and shared crates under `foreman/`.
- Python workspace for voice and research MCP servers under `mcp-servers/python/` using pyproject/uv.
- Dev task runner via `just`/`make` (build/run/test/format).

## Security Posture

- Local-first; explicit approvals for risky operations; quarantined downloads; dry-run everywhere feasible.
- Never store secrets in logs; redact sensitive paths by policy.

## Future Phases

- Mobile PWA bridge, advanced debate/reporting UI, additional MCP servers (calendar, email), multi-profile/home setup.
