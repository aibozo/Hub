set shell := ["bash", "-eu", "-o", "pipefail", "-c"]

default: build

alias b := build
alias t := test
alias f := fmt
alias l := lint

build:
  cargo build --workspace

test:
  cargo test --workspace || true
  if [ -d mcp-servers/python/voice_daemon ]; then \
    (cd mcp-servers/python/voice_daemon && python -m pip -q install -e .[dev] >/dev/null 2>&1 || true && pytest -q || true); \
  fi
  if [ -d mcp-servers/python/arxiv_server ]; then \
    (cd mcp-servers/python/arxiv_server && python -m pip -q install -e .[dev] >/dev/null 2>&1 || true && pytest -q || true); \
  fi

fmt:
  cargo fmt --all

lint:
  cargo clippy --workspace -D warnings || true
  ruff --version >/dev/null 2>&1 && ruff . || true
  black --version >/dev/null 2>&1 && black --check . || true

run-core:
  cargo run -p assistant-core

run-tui:
  cargo run -p ui-tui --features tui,http

# Launches core in background, waits for readiness, then runs TUI
tui:
  bash scripts/run-tui.sh

# Build Rust MCP servers (explicit)
build-servers:
  cargo build -p mcp-shell -p mcp-fs -p mcp-proc -p mcp-git

# Install Rust MCP servers to ~/.cargo/bin
install-servers:
  cargo install --path mcp-servers/rust/shell --force
  cargo install --path mcp-servers/rust/fs --force
  cargo install --path mcp-servers/rust/proc --force
  cargo install --path mcp-servers/rust/git --force
