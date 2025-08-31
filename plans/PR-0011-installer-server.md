# PR-0011 — Installer Server

Summary: Implement an MCP server to plan/explain/apply package installs across apt/snap/flatpak/pip/cargo with dry-run and policy-aware approvals.

Dependencies: PR-0003 (policy), PR-0002.

Deliverables

- `mcp-servers/python/installer_server/` with `pyproject.toml` and `__main__.py`.
- Tools: `plan_install(pkg, manager?)`, `explain_install(plan_id)`, `dry_run(plan_id)`, `apply_install(plan_id, approve_token)`.
- Manifest `config/tools.d/installer.json`.

Implementation Notes

- Normalize package names; detect manager if not provided; propose commands with exact args.
- Explain: source, hash, vendor, repository, and safety notes; include dry-run output.
- Apply executes only with a valid approval token; record events and logs.

Wiring Checklist

- Wire policy approvals: core must validate an approval token before `apply_install` runs.
- TUI Approvals UI (PR-0008) supports approving installer plans; show plan/explain cards.
- Record provenance and dry-run output, retrievable via core `explain` endpoint.

Tests

- Unit: plan resolution for sample packages; dry-run parsing; approval token validation path.
- Integration: with mocks for package managers, simulate an install flow.

Acceptance Criteria

- Plan/explain/dry-run/apply cycle works end-to-end with approvals; no writes without approval.
- TUI can approve a pending installer plan and see apply results; events and provenance are recorded in memory.

References

- docs/TOOLS.md, docs/POLICY.md, docs/ARCHITECTURE.md (System mgmt)

Out of Scope

- GUI for approvals (handled in TUI PRs).
