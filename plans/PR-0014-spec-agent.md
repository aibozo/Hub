# PR-0014 — Spec Mode Agent

Summary: Implement the `mcp-spec` server that scans a repo, asks targeted questions for unknowns, and produces plan/architecture/integration docs.

Dependencies: PR-0001.

Deliverables

- `mcp-servers/python/spec_server/` with tools: `scan(dir)`, `questions(context)`, `generate(docs=[plan, architecture, integration])`.
- Emits artifacts under `storage/artifacts/spec/` and can optionally open a PR/patch (approval-gated via core).

Wiring Checklist

- Register manifest `config/tools.d/spec.json`; core proxy endpoint `/api/spec/*`.
- TUI `/spec <dir>` triggers a scan and displays generated docs; optionally opens a core approval card for patch application.
- Ensure generated docs reference `docs/` and `plans/` conventions.

Implementation Notes

- Whitelist paths; respect `.gitignore`; avoid large binary files.
- Template outputs that reference `docs/ARCHITECTURE.md` and `plans/*` conventions.

Tests

- Unit: scanning excludes ignored paths and large binaries; question generation for missing configs.

Acceptance Criteria

- Running `generate` on a small repo yields plan.md and architecture.md with cross-references.
- TUI command runs the scan and shows a link to generated artifacts; patch application is approval-gated via core.

References

- docs/ARCHITECTURE.md (Spec Help Mode), docs/TOOLS.md

Out of Scope

- Automatic PR creation without approvals.
