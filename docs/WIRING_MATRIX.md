# Wiring Matrix

This matrix specifies, for each subsystem, where it is implemented and where it is wired into the rest of the system. Use it to avoid leaving stubs behind.

- Assistant Core
  - Implemented: PR-0002
  - Wires to: Policy (PR-0003), Memory (PR-0004), System Map (PR-0005), MCP Servers (PR-0006), Voice (PR-0007), Schedulers (PR-0009), Telemetry (PR-0016)

- Policy/Gatekeeper
  - Implemented: PR-0003
  - Wires to: Core exec path & MCP client (PR-0003), MCP servers enforcement (PR-0006), Installer approvals (PR-0011), TUI approvals UI (PR-0008 updates, PR-0017)

- Memory Plane
  - Implemented: PR-0004
  - Wires to: Core event logging (PR-0004), Tasks API (PR-0002 update), Context packer in planner (PR-0004), TUI Memory screen (PR-0008)

- System Map
  - Implemented: PR-0005
  - Wires to: Context packer (PR-0004), API `/api/system_map` (PR-0002 update), TUI Settings/Info (PR-0008)

- MCP Core Servers (shell/fs/proc/git)
  - Implemented: PR-0006
  - Wires to: Core MCP registry (PR-0006), Policy checks (PR-0003), TUI Tools panel (PR-0008)

- Codex MCP (CLI) Integration
  - Implemented: PR-XXXX (this change)
  - Wires to: Tools registry (adapter `mcp-codex`), Core API `/api/codex/{new,continue}`. TUI Codex tab pending (follow-up PR).

- Voice Daemon (TTS/STT/VAD/Wake)
  - Implemented: PR-0007
  - Wires to: Core audio-out client and WS (PR-0007), TUI `/voice` controls (PR-0008)

- Realtime Bridge (V2V)
  - Implemented: PR-0023
  - Wires to: Tools Manager + Gatekeeper (PR-0006/PR-0003), TUI controls (PR-0020)

- Realtime Audio I/O
  - Implemented: PR-0018
  - Wires to: Realtime Bridge (PR-0016), Device I/O, Metrics

- Wake Sentinel
  - Implemented: PR-0019
  - Wires to: Realtime start/stop endpoints (PR-0016), TUI toggles (PR-0020)

- TUI Realtime Wiring
  - Implemented: PR-0020
  - Wires to: Core `/api/realtime/*` (PR-0016), Help overlay & Keymap

- V2V↔T2T History Sharing
  - Implemented: PR-0021
  - Wires to: Chat sessions (Core), Memory events

- Realtime WebRTC Transport (optional)
  - Implemented: PR-0022
  - Wires to: Realtime Bridge transport layer (PR-0016)

- Schedulers & Briefs
  - Implemented: PR-0009
  - Wires to: Research servers (PR-0010), Artifacts + Memory (PR-0004), TUI Tasks or Briefs view (PR-0008 update)

- Research (arXiv/News)
  - Implemented: PR-0010
  - Wires to: Scheduler jobs (PR-0009), Tools registry (PR-0006 update)

- Installer Server
  - Implemented: PR-0011
  - Wires to: Policy approvals (PR-0003), Core approval flow (PR-0003), TUI approvals UI (PR-0008 update, PR-0017)

- Websearch Server
  - Implemented: PR-0012
  - Wires to: Debate (PR-0013), Spec (PR-0014), Tools registry (PR-0006 update), TUI quick command (PR-0008 update)

- Debate Agent
  - Implemented: PR-0013
  - Wires to: TUI `/debate` (PR-0008 update), Artifacts + Memory (PR-0004)

- Spec Mode Agent
  - Implemented: PR-0014
  - Wires to: TUI `/spec` (PR-0008 update), Artifacts (PR-0004), optional patch/PR via core approvals (PR-0003)

- CI & Testing
  - Implemented: PR-0015
  - Wires to: All crates/packages; uploads artifacts on failure

- Telemetry & Health
  - Implemented: PR-0016
  - Wires to: Core and all servers; TUI health indicators (PR-0008 update)

- Final Integration
  - Implemented: PR-0017
  - Wires to: Ensures all above are connected, removes temporary stubs/feature flags, verifies end-to-end tests.
