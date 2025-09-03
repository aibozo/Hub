# Policy and Safety Gates

This document defines the Gatekeeper policy model, approval flow, and configuration overlays. All MCP tool calls and subprocess executions must pass through the policy engine.

## Command Classes

- Safe: read-only actions (list, open, fetch, search).
- Warn: writes within user home; package installs from trusted repos.
- Block-by-default: `sudo`, system-level changes, deletes outside whitelisted paths, firewall/network changes.

## Approval Flow

1. Tool proposes an action with a plan card: why, exact command, paths, inputs, sources.
2. Core evaluates rules (YAML). If risky → HOLD with rationale.
3. User approves/denies from TUI (or voice confirmation). Optional Explain-This shows provenance (source, hash, vendor, dry-run output).
4. On approval, core executes under supervision: pty wrapper, timeouts, CPU/mem caps, stdout/err capture to Event log.

## Dry-Run and Quarantine

- Executors must support dry-run where possible (`apt-get -s`, `pip install --dry-run`), otherwise simulate.
- All downloads land in `~/Foreman/Quarantine/<sha256>` until approved and scanned.

## Policy Files

- Location: `config/policy.d/`
- Precedence: lexicographic by filename (e.g., `00-defaults.yaml` < `10-local-overrides.yaml`).
- Schema (YAML):

```yaml
protect_paths:
  - "/"
  - "/etc"
  - "/usr"
  - "/var"
  - "/boot"
write_whitelist:
  - "~/"
  - "/mnt/data/projects"
require_approval:
  - sudo
  - apt
  - snap
  - flatpak
  - pip
  - cargo
  - "rm -rf"
  - pkill
  - iptables
env_allowlist:
  - PATH
  - HOME
  - LANG
limits:
  wall_time_sec: 120
  cpu_percent: 80
  mem_mb: 2048
log_redactions:
  - pattern: ".*TOKEN=.*"
    replace: "TOKEN=***"
```

## Enforcement Points

- MCP client: pre-flight tool inputs; forbid dangerous paths; attach `--dry-run` for plan/explain.
- Subprocess runner: sandbox cwd, ensure path/policy checks, pass only allowlisted env vars, apply timeouts/resource limits.
- FS writes: require explicit allow under `write_whitelist`.

### Agent Write Approvals

- The following actions require approval by default (see `config/policy.d/30-codex.yaml`):
  - `apply_patch` (core `patch.apply` tool)
  - `git commit`
- Core evaluates `ProposedAction { command, writes, paths[] }` and, if not `Allow`, surfaces an ephemeral prompt with reasons and affected files.
- Upon approval, a token is issued and the original action is retried with `approval_id` + `approve_token`.
- The TUI shows the prompt inline and can approve/deny, or fetch an Explain‑This card for provenance.

## Explain-This (Provenance)

- For installs: show package origin, signature/hash, vendor links, and dry-run output.
- For deletes/moves: list target paths and whether they intersect `protect_paths`.

## Auditing

- All actions are appended to Event log with unique approval tokens, timestamps, and normalized command lines.
