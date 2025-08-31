# PR-0024 — Realtime Tool Bridge & `end_call`

Summary: Expose all core tools to the realtime session with gatekeeper enforcement and approvals. Implement the synthetic `end_call` tool to terminate V2V and return to T2T mode cleanly.

Dependencies: PR-0023 (realtime bridge)

## Deliverables
- Tool schema generator from `config/tools.d/*.json` manifests to realtime JSON Schemas.
- Tool bridge: handle `tool.call` → gatekeeper → Tools Manager → `tool.output`.
- Synthetic `end_call` tool that shuts down the session and flips mode.
- Tests with a mock realtime server covering tool calls and `end_call`.

## Implementation
- Helper: `tools::to_json_schemas()` returns a vector of tool schema objects + `end_call`.
- In `realtime.rs` event loop (PR-0016), match `tool.call` events, construct `ProposedAction`, and route through gatekeeper approvals before execution.
- On `end_call`, call `RealtimeManager::stop()` and send an acknowledgment (e.g., a `tool.output` with `{ ok: true }`).

## Tests
- Unit: schema generation parity with manifests; param typing fidelity.
- Integration (mock server): server requests a known tool, returns expected result; `end_call` leads to inactive status.

## Acceptance Criteria
- Realtime session advertises tools matching manifests; tool calls execute with approvals when required.
- `end_call` reliably terminates the session and status reflects inactive.

## Wiring Checklist
- Gatekeeper and approvals remain enforced; provenance events logged.
- Errors are surfaced as `tool.output` error objects; no panics.
