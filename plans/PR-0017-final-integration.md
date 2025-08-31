# PR-0017 — Final Integration & Wiring Completion

Summary: Remove any temporary stubs/feature flags, verify all subsystems are fully wired per the wiring matrix, and add end-to-end tests that cover voice, memory, MCP tools, approvals, and schedulers.

Dependencies: PR-0001 through PR-0016.

Deliverables

- Remove temporary stub implementations and feature flags introduced earlier.
- Verify and, if needed, complete the following wiring:
  - Policy enforcement on all MCP calls and subprocess executions.
  - Memory event logging for core API calls, MCP calls, scheduler jobs, and artifact writes.
  - System map digest inclusion in context packs and exposure via API/TUI.
  - MCP server registration and reachability from core; TUI tools panel commands.
  - Voice WS connection from core + audio-out; TUI `/voice` test and barge-in stop.
  - Scheduler jobs producing artifacts; TUI shows latest briefs or task entries.
  - Approvals UI path in TUI end-to-end with installer apply.
  - Telemetry/health endpoints surfaced and TUI indicators wired.
- End-to-end tests:
  - Voice TTS roundtrip (mock engine acceptable) with barge-in.
  - Installer dry-run and approved apply (no-op backend) path with approvals UI.
  - Memory context pack includes system map + task digest; expansion handle works.
  - Scheduler runs a mocked brief and creates artifacts.

Wiring Checklist

- Confirm every item in `docs/WIRING_MATRIX.md` has a corresponding code path and test.
- Remove any TODOs referencing deferred wiring.
- Ensure `docs/ARCHITECTURE.md` and `AGENTS.md` reflect the final wiring.

Acceptance Criteria

- All checklist items are complete; end-to-end tests pass locally and in CI.
- No remaining stubs or unreferenced feature flags.

References

- docs/WIRING_MATRIX.md, docs/ARCHITECTURE.md, docs/TESTING.md, AGENTS.md

