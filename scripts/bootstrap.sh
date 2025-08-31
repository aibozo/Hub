#!/usr/bin/env bash
set -euo pipefail
echo "[bootstrap] Setting up dev environment..."
echo "- Ensuring storage/ directory exists"
mkdir -p storage/{artifacts,indices,quarantine,logs}
echo "- (Optional) Create Python venvs in mcp-servers/python/*"
echo "- (Optional) Install Rust components via rustup (clippy/rustfmt)"
echo "Done. See AGENTS.md for build/run/test."

