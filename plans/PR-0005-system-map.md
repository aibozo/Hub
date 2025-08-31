# PR-0005 — System Map Scanner + Digest

Summary: Implement machine inventory scanning and a compact pinned digest with map URIs for detail expansion.

Dependencies: PR-0002.

Deliverables

- `apps/assistant-core/src/system_map/`:
  - `mod.rs`, `model.rs` (types), `scan.rs` (enumerators), `digest.rs` (200–400 token digest).
- Persist `storage/map.json` and emit a `SystemMapUpdated` event on changes.

Wiring Checklist

- Add `/api/system_map` and `/api/system_map/digest` endpoints in core.
- Feed the digest into `context_pack.rs` so it appears in every pack by default (PR-0004 integration point).
- Expose a TUI Settings/Info pane (PR-0008) to render the digest and link to map:// resolvers.

Implementation Notes

- Enumerate hardware/OS/runtimes/package managers/apps/dev envs/network (limited), using non-invasive commands and timeouts.
- Compute digest that names components and includes approved interaction rules.
- Provide resolver for `map://packages`, `map://emulators`, `map://worktrees` URIs.

Tests

- Unit: digest length within bounds; map→digest deterministic given a fixed map.
- Integration: on first run, create `map.json`; on changes, emit an Event and update digest.

Acceptance Criteria

- `map.json` exists and digest is available to the context packer.
- Core endpoints serve the map and digest; TUI has a placeholder that consumes these endpoints.

References

- docs/SYSTEM_MAP.md, docs/ARCHITECTURE.md (System Map), docs/TESTING.md

Out of Scope

- Uploading map to external services; deep network scans.
