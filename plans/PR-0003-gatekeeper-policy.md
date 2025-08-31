# PR-0003 — Gatekeeper Policy Engine

Summary: Implement the safety/policy layer with command classification, approval flow primitives, dry-run integration points, and provenance explainers.

Dependencies: PR-0002.

Deliverables

- `apps/assistant-core/src/gatekeeper/`:
  - `mod.rs`
  - `policy.rs`: load/merge YAML overlays from `config/policy.d/*.yaml` into strongly-typed rules.
  - `approvals.rs`: approval token generation/validation, queues, expirations.
  - `provenance.rs`: explainers for installs/deletes with dry-run hooks.
- `crates/foreman-policy/`: rule evaluation library consumed by core and servers.
- `config/policy.d/00-defaults.yaml` and `10-local-overrides.yaml` examples.

Wiring Checklist

- Integrate `GateCheck` into assistant-core subprocess execution path and MCP client pre-flight in `mcp_client.rs`.
- Add approval queue endpoints to core API: `POST /api/approvals` (create), `GET /api/approvals`, `POST /api/approvals/{id}/approve|deny`.
- Ensure MCP core servers (PR-0006) call back into core for approval validation when attempting gated actions.
- Add Explain-This endpoint `GET /api/explain/{action_id}` returning provenance details.

Implementation Notes

- Command classes: Safe/Warn/Block-by-default per `docs/POLICY.md`.
- Provide `GateCheck` API: input normalized command + paths + intent → classification + rationale + required approvals.
- Dry-run adapter trait that tools implement; for now provide stubs.
- `ExplainThis`: structure references to sources, hashes, vendor/package origins, and dry-run output mapping.

Tests

- Unit: policy merge precedence; classification for sample commands (apt/sudo/rm -rf/home writes).
- Unit: approval token lifecycle.
- Unit: provenance output for a fixture install plan.

Acceptance Criteria

- Given a risky command, `GateCheck` returns HOLD with clear rationale.
- Approvals can be created, listed, and invalidated; tokens are unique and time-bounded.
- Core API exposes approvals and explain endpoints; MCP client and exec runner enforce policy.

References

- docs/POLICY.md, docs/ARCHITECTURE.md (Policy section)

Out of Scope

- UI for approvals (handled by TUI later); actual subprocess execution.
