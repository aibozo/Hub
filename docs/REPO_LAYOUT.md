# Repository Layout (Planned)

The project uses a Rust workspace for core + TUI + shared crates, and a Python workspace for voice and research MCP servers. This document is the authoritative map for directories and their purposes.

```
foreman/
├─ Cargo.toml                      # Workspace root (Rust)
├─ rust-toolchain.toml             # Pin toolchain
├─ Justfile                        # dev tasks (build/run/test/format)
├─ Makefile                        # optional; mirrors Justfile targets
├─ .editorconfig
├─ .gitignore
├─ .env.example
├─ README.md
├─ LICENSE
│
├─ apps/
│  ├─ assistant-core/              # Rust orchestrator daemon (bin)
│  └─ ui-tui/                      # Rust TUI (bin)
│
├─ crates/                         # Shared Rust libs (clean seams)
│  ├─ foreman-types/
│  ├─ foreman-config/
│  ├─ foreman-policy/
│  ├─ foreman-memory/
│  ├─ foreman-mcp/
│  ├─ foreman-exec/
│  ├─ foreman-system-map/
│  └─ foreman-telemetry/
│
├─ mcp-servers/
│  ├─ rust/{shell,fs,proc,git,emu,steam}/
│  └─ python/{voice_daemon,arxiv_server,news_server,websearch_server,installer_server,spec_server,debate_server}/
│
├─ config/
│  ├─ foreman.toml                 # main config
│  ├─ policy.d/{00-defaults.yaml,10-local-overrides.yaml}
│  ├─ tools.d/{shell.json,fs.json,proc.json,git.json,arxiv.json,news.json,websearch.json,installer.json,emu.json,steam.json}
│  ├─ schedules.toml               # cron-like job times
│  └─ tui.toml                     # theme, keymap
│
├─ storage/
│  ├─ sqlite.db                    # event log + atoms
│  ├─ indices/                     # tantivy + vector index files
│  ├─ artifacts/                   # reports, cached PDFs, screenshots
│  ├─ quarantine/                  # downloaded files awaiting approval
│  └─ logs/
│
├─ docs/
│  ├─ ARCHITECTURE.md
│  ├─ REALTIME.md
│  ├─ POLICY.md
│  ├─ MEMORY.md
│  ├─ SYSTEM_MAP.md
│  ├─ TOOLS.md
│  ├─ TESTING.md
│  └─ ADR/
│
├─ scripts/
│  ├─ bootstrap.sh
│  ├─ dev-run.sh
│  ├─ install-rust.sh
│  ├─ install-python.sh
│  ├─ migrate.sh
│  └─ pack-assets.sh
│
├─ .github/workflows/{ci.yml,lint.yml}
└─ pwa/                             # (later) Next.js bridge (optional)
```

Workspace `Cargo.toml` members:

```
[workspace]
members = [
  "apps/assistant-core",
  "apps/ui-tui",
  "crates/foreman-types",
  "crates/foreman-config",
  "crates/foreman-policy",
  "crates/foreman-memory",
  "crates/foreman-mcp",
  "crates/foreman-exec",
  "crates/foreman-system-map",
  "crates/foreman-telemetry",
  "mcp-servers/rust/shell",
  "mcp-servers/rust/fs",
  "mcp-servers/rust/proc",
  "mcp-servers/rust/git",
  "mcp-servers/rust/emu",
  "mcp-servers/rust/steam",
]
resolver = "2"
```
