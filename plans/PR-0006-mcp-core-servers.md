# PR-0006 — Core MCP Servers (shell/fs/proc/git)

Summary: Implement baseline MCP servers in Rust with path policy enforcement, env allowlists, timeouts, and dry-run support where applicable.

Dependencies: PR-0003 (policy hooks), PR-0002.

Deliverables

- `mcp-servers/rust/shell/` with tools: `exec`, `list_dir`, `read_file`, `write_file` (write gated), `which`.
- `mcp-servers/rust/fs/` with tools: `stat`, `list`, `read`, `write` (gated), `mkdir`, `move` (gated).
- `mcp-servers/rust/proc/` with tools: `list`, `kill` (gated), `renice` (gated).
- `mcp-servers/rust/git/` with tools: `status`, `branches`, `worktrees`, `diff_summary`, `create_branch`, `switch`.
- Manifests under `config/tools.d/{shell,fs,proc,git}.json`.

Implementation Notes

- Transport: stdio. Protocol types via `crates/foreman-mcp`.
- Enforce policy at entry: normalize inputs, path checks, env filtering via `foreman-policy`.
- Dry-run: no-op for read-only; simulate effects for write operations.

Wiring Checklist

- Register all manifests in `config/tools.d/` and ensure assistant-core loads them at startup.
- Establish stdio transports for each server and health-check them on core boot.
- Route TUI Tools panel (PR-0008) commands via core: `/api/tools/{server}/{tool}` proxy endpoint with policy pre-flight.
- Enforce policy checks on every tool invocation via `foreman-policy` (PR-0003 integration).

Tests

- Unit: path policy enforcement for protected paths; env allowlist behavior.
- Integration: create a temp workspace; run `list_dir`, `read_file`; verify writes require approval in core wiring (to be completed later).

Acceptance Criteria

- Servers run and respond to basic tool invocations.
- No operation writes outside `write_whitelist` without explicit approval tokens (core integration later ensures this).
- Core can invoke each server via API, and TUI tools can call at least one read-only command end-to-end.

References

- docs/TOOLS.md, docs/POLICY.md, docs/ARCHITECTURE.md (MCP)

Out of Scope

- Non-core servers (arXiv/news/etc.).
