# PR-0107 — Multiagent Research Scaffold (Workers & Orchestration)

Summary: Add a minimal, policy‑compliant multiagent scaffold that can fan‑out work (ranking, dedup, per‑paper structured notes) across lightweight worker agents (e.g., `gpt-5-mini`), then synthesize via a judge/aggregator step. All context goes through the packer; hard budgets enforced to prevent prompt bloat.

Dependencies: PR-0104 (packer), PR-0105 (bundles)

Deliverables
- Core module `apps/assistant-core/src/research/agents/`:
  - `worker.rs`: stateless worker that receives a `PaperMeta` slice and returns normalized notes (claims, methods, caveats). Uses `/api/chat/complete` under strict token limits.
  - `judge.rs`: aggregator that merges worker notes and selects top‑K with rationales.
  - `orchestrator.rs`: assigns batches (sharded) within global budget; retries; collects results deterministically.
- Config: `[research.agents]` limits (max concurrent workers, per‑worker tokens, global cap).
- Policy: no external processes; all tools/LLM calls go through core.
- Optional feature flag `multiagent` to toggle the path while keeping deterministic tests.

Acceptance Criteria
- Given a `ReportBundle`, workers produce compact JSON notes within configured budgets; aggregator returns a stable top‑K list and merged highlights.
- No change to TUI in this PR; pipeline selectable via config or feature.
- Deterministic in tests (fixed seeds and sampling order; model calls mocked).

Implementation Notes
- Keep the worker prompts short and structured; avoid freeform text.
- Respect research packer budgets as a superset constraint.
- Design agents to be gateway‑agnostic (extensible to future research servers).

Tests
- Unit: sharding logic, budget enforcement, deterministic ordering.
- Integration: mock chat completions and verify merged outputs.

Wiring Checklist
- Modules added; config surfaced; feature flag available.
- All calls traverse policy/approvals as usual; no direct subprocesses.
- Docs: `docs/ARCHITECTURE.md` multiagent overview; `docs/TESTING.md` mocking strategy.
- WIRING_MATRIX: mark “Multiagent research scaffold — implemented; UI wiring in PR‑0108”.

