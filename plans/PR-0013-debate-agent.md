# PR-0013 — Debate Agent (Deep Argue)

Summary: Implement an MCP server orchestrating two stance agents and a judge. Produces a structured debate report with citations, concessions, and unresolved points.

Dependencies: PR-0001.

Deliverables

- `mcp-servers/python/debate_server/` with tools: `debate(topic, stance_a, stance_b, rounds, search_budget)` and `report(debate_id)`.
- Shared citation pool + facility to add sources mid-debate via websearch.
- Output report artifact (markdown + JSON log) under `storage/artifacts/debates/`.

Implementation Notes

- Enforce source-grounding for factual claims; judge penalizes uncited assertions.
- Track concessions explicitly and include them in the final report.

Wiring Checklist

- Register manifest `config/tools.d/debate.json`.
- TUI command `/debate <topic>` kicks off a debate via core proxy; progress updates appear in chat screen.
- Write final report to artifacts; append summary atom; expose report via TUI link.

Tests

- Unit: state machine across rounds; citation injection; report schema validation.

Acceptance Criteria

- Running a small debate yields a report with required sections and citations.
- TUI command runs end-to-end and shows a link to the produced report.

References

- docs/TOOLS.md, docs/ARCHITECTURE.md (Deep Argue)

Out of Scope

- Advanced UI; long-running debates with external LLM APIs.
