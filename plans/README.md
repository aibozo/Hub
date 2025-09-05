# PR Plan Index and Sequence

Each plan is a self-contained starting context for a focused PR. Follow the order unless prior work is already complete. Reference `docs/*` for shared architecture.

1. PR-0001 — Repo Bootstrap & Scaffolding
2. PR-0002 — assistant-core Skeleton
3. PR-0003 — Gatekeeper Policy Engine
4. PR-0004 — Memory Plane Base (SQLite + Events + Atoms)
5. PR-0005 — System Map Scanner + Digest
6. PR-0006 — Core MCP Servers (shell/fs/proc/git)
7. PR-0007 — Voice Daemon (TTS/STT/VAD/Wake)
8. PR-0008 — TUI Skeleton (ratatui)
9. PR-0009 — Schedulers & Daily Briefs
10. PR-0010 — Research Servers (arXiv/News)
11. PR-0011 — Installer Server
12. PR-0012 — Websearch Server
13. PR-0013 — Debate Agent
14. PR-0014 — Spec Mode Agent
15. PR-0015 — CI + Testing Pipeline
16. PR-0016 — Telemetry & Health
17. PR-0017 — Final Integration & Wiring Completion

Realtime Series (post-0017):
- PR-0023 — Realtime Bridge (V2V)
- PR-0024 — Realtime Tool Bridge & `end_call`
- PR-0018 — Realtime Audio I/O
- PR-0019 — Wake Sentinel
- PR-0020 — TUI Realtime Wiring
- PR-0021 — History Sharing (V2V↔T2T)
- PR-0022 — Realtime WebRTC Transport (optional)

See `docs/WIRING_MATRIX.md` for subsystem ↔ wiring PR mapping.

Web UI Series (local browser app):

1. PR-0200 — Web UI Enablement: CORS & Policy
2. PR-0201 — Web UI Scaffold, Theme, Health
3. PR-0202 — Web UI Chat MVP (Sessions, POST‑SSE Streaming)
4. PR-0203 — Web UI Approvals Inline (Gatekeeper Flow)
5. PR-0204 — Web UI Agents (List, Detail, Actions, Use in Chat)
6. PR-0205 — Web UI Research (Tasks, Drawer, Brief Triggers)
7. PR-0206 — Web UI Settings & Theming
8. PR-0207 — Web UI Final Integration & Wiring Completion
