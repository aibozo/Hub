# PR-0015 — CI + Testing Pipeline

Summary: Establish CI workflows for lint/build/test across Rust and Python, with caches and artifact uploads on failure.

Dependencies: PR-0001 (bootstrap), and partial stubs from earlier PRs to make builds meaningful.

Deliverables

- `.github/workflows/ci.yml`: matrix for Linux; steps for fmt/clippy/build/test (Rust) and ruff/black/pytest (Python).
- `.github/workflows/lint.yml`: quicker lint-only workflow on PR open.
- Pre-commit hooks (optional) for devs.

Implementation Notes

- Cache cargo registry and target; cache Python `.venv`/`uv`.
- On failure, upload `target/test-output` and test logs as artifacts.

Tests

- Validate workflow locally with `act` (optional) or by opening a draft PR.

Acceptance Criteria

- CI passes for the current scaffold; clear logs and actionable failures.

References

- docs/TESTING.md

Out of Scope

- Release workflows.

