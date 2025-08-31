# System Map

The System Map provides a machine inventory and an approved interaction guide, surfaced as a compact pinned digest plus detail pages resolvable locally.

## Inventory Scanner

- Runs daily or on detected changes; outputs `storage/map.json`.
- Captures: hardware (CPU/GPU/RAM/disks/audio), OS (distro/kernel/services/shells/timezone), runtimes (Python/Rust/Node/CUDA/Java), package managers (apt/snap/flatpak/pip/cargo/conda), installed apps/CLIs (emulators, Steam, browsers, editors, docker/podman), dev envs (git repos, worktrees, language servers), and selected network info (interfaces, local domains, limited open ports, local services).

## Pinned Digest

- ~200–400 token summary including component names and approved interaction rules per component.
- Inline links use local URIs: `map://packages`, `map://emulators`, `map://worktrees`, which are resolved by core at runtime and only expanded into model context on demand.

## Scanning Strategy

- Prefer non-invasive commands (e.g., `lsb_release`, `/proc`, `which`, `--version` probes) with timeouts; avoid deep system scans by default.
- Cache previous map and diff for changes; emit events for significant deltas (new GPU driver, new package managers, etc.).

## Privacy and Policy

- Respect policy redactions; exclude sensitive paths/configs unless explicitly expanded by the user.
- Network info limited to local interfaces/services; never enumerate external hosts/scans.

