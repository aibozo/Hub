# ArXiv Research — PR Series Overview

This folder contains a focused PR series to integrate an arXiv research capability end‑to‑end: MCP server, core research framework, scheduler brief generation, and a TUI research workspace. The series ensures context funneling, modular addition of future research gateways, and a path to multi‑agent expansion.

Two tracks are provided; choose one to ship first. The Rust track is preferred and becomes canonical once merged. The last PR closes all wiring per Wiring Policy.

- Track A (Quick Win): PR-A100 — Python arXiv server wiring and scheduler brief using existing stub. (Keeps scope small; replace later by Rust.)
- Track B (Preferred): PR-0101..0108 — Rust MCP server and full framework + UI.

Status and wiring are reflected in docs/WIRING_MATRIX.md during the PRs.

## Track B — PRs

- PR-0101 — Rust arXiv MCP: crate scaffold + tool schemas
- PR-0102 — ArXiv client + tools: search/get/download (+ tests)
- PR-0103 — Core integration: manifest, autostart, policy, storage
- PR-0104 — Research framework + context packer (budgets, funnels)
- PR-0105 — Scheduler job: arxiv_brief (reports + artifacts)
- PR-0106 — TUI: Research → arXiv screen (search/results/report)
- PR-0107 — Multiagent scaffold: worker orchestration & limits
- PR-0108 — Final integration: wiring matrix closed + E2E tests

## Track A — PRs

- PR-A100 — Quick Win: use Python arXiv server today; implement scheduler brief; UI leverages existing Tools panel (optional minimal Research screen).

Acceptance Criteria across the series:
- All runtime writes under `storage/` only; policy allowlists updated accordingly.
- Deterministic tests; arXiv API mocked for unit/integration.
- No indefinite stubs; follow‑ups explicitly listed and closed by PR‑0108.
- TUI is keyboard navigable and documents keybindings in help overlay.
- Tools appear and operate via `/api/tools` and scheduler endpoints; artifacts land under `storage/briefs/arxiv/` and are recorded in memory.

