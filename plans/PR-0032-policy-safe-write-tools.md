# PR-0032 — Policy‑Safe Write Tools (patch.apply, git.commit)

- Owner: core/gatekeeper
- Status: Draft
- Depends on: PR-0031
- Wires: ToolsManager, Gatekeeper approvals

## Summary
Add in‑core tools for safe code changes and commits, pre‑flighted by the Policy engine with ephemeral approval on yellow/red. This ensures all mutations flow through the gatekeeper and are auditable.

## Deliverables
- New tools under `apps/assistant-core/src/tools.rs`:
  - `patch.apply`: apply unified diff with file count/size caps and allowed path prefixes.
  - `fs.write_text`: create/overwrite text files in allowed roots.
  - `git.branch`, `git.add`, `git.commit`: local actions only; no push.
- Policy overlay updates in `config/policy.d/30-codex.yaml`:
  - Ensure `apply_patch`, `git commit`, `git apply` require approval unless within explicit safe scope.
- API: extend policy check endpoint docs/examples if needed.
- Unit tests: approval required/denied cases; success with valid token.

## Scope
- In‑core tools + policy. No TUI changes.

## Non‑Goals
- Network operations (push/PR). Covered later if needed.

## Acceptance Criteria
- Attempting `patch.apply` without approval on protected path is blocked (Warn/Hold).
- With approved token, operation succeeds and event is recorded with provenance.
- Git commit blocked unless within allowed scope or token provided.

## Tests
- `patch_apply_policy_blocks_then_allows`.
- `git_commit_requires_token`.

## Wiring Checklist
- CTR uses these tools for Apply/Commit (PR-0033) — OPEN
- TUI approvals drawer shows these approvals (PR-0034) — OPEN

## Rollback Plan
- Tools can remain resident; policy keeps them inert.

## Risks
- None; strictly reduces risk surface.

