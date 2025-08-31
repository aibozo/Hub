# PR-0016 — Telemetry & Health

Summary: Add structured logging, metrics, and health endpoints across core and servers.

Dependencies: PR-0002.

Deliverables

- Core: `tracing` subscriber with JSON logs; `GET /metrics` Prometheus endpoint; `GET /ready` and `/health`.
- Servers: basic health endpoints or ping tool; include minimal metrics.
- Centralize telemetry setup in `crates/foreman-telemetry`.

Implementation Notes

- Per-task spans; record MCP round-trip latencies; expose scheduler job metrics.

Tests

- Unit: metrics registry increments on simulated events.
- Integration: health endpoints return 200; metrics contain expected keys.

Acceptance Criteria

- Running core and servers exposes health and metrics; logs are structured and redact secrets by policy.

References

- docs/ARCHITECTURE.md (Observability), docs/POLICY.md (redactions)

Out of Scope

- External collectors/deployments.

