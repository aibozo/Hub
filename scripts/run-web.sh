#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
API_BASE="http://127.0.0.1:6061"

# Args
#   -r | --restart-core  Kill any running assistant-core and start a fresh one
#   --core-release       Run assistant-core with --release (default: debug)
#   --mock-chat          Run assistant-core with CHAT_MOCK=1 (server streams mock tokens)
RESTART_CORE=0
CORE_RELEASE=0
MOCK_CHAT=0
while [[ $# -gt 0 ]]; do
  case "$1" in
    -r|--restart-core) RESTART_CORE=1; shift ;;
    --core-release) CORE_RELEASE=1; shift ;;
    --mock-chat) MOCK_CHAT=1; shift ;;
    -h|--help)
      echo "Usage: $0 [--restart-core|-r] [--core-release]"; exit 0 ;;
    *) echo "[hub-web] Unknown option: $1"; shift ;;
  esac
done

echo "[hub-web] Starting assistant-core..."

# Optionally kill any existing core
if [[ "$RESTART_CORE" == "1" ]]; then
  echo "[hub-web] --restart-core: stopping existing assistant-core (if any)…"
  # Prefer killing by port to avoid over-matching
  if command -v lsof >/dev/null 2>&1; then
    PIDS=$(lsof -ti :6061 || true)
    if [[ -n "${PIDS}" ]]; then
      echo "$PIDS" | xargs -r kill || true
      sleep 0.4
      # Force kill if still alive
      PIDS2=$(lsof -ti :6061 || true)
      if [[ -n "${PIDS2}" ]]; then echo "$PIDS2" | xargs -r kill -9 || true; fi
    fi
  else
    # Fallback: kill by name (less precise)
    pkill -f assistant-core || true
  fi
fi

# Start core if not listening
if ! nc -z 127.0.0.1 6061 2>/dev/null; then
  (
    cd "$ROOT_DIR"
    if [[ "$CORE_RELEASE" == "1" ]]; then
      echo "[hub-web] Running assistant-core (release)…"
      if [[ "$MOCK_CHAT" == "1" ]]; then CHAT_MOCK=1; fi
      env RUST_LOG=info CHAT_MOCK="${CHAT_MOCK:-}" cargo run -p assistant-core --bin assistant-core --release
    else
      echo "[hub-web] Running assistant-core (debug)…"
      if [[ "$MOCK_CHAT" == "1" ]]; then CHAT_MOCK=1; fi
      env RUST_LOG=info CHAT_MOCK="${CHAT_MOCK:-}" cargo run -p assistant-core --bin assistant-core
    fi
  ) &
  CORE_PID=$!
  echo "[hub-web] assistant-core PID: $CORE_PID"
else
  echo "[hub-web] assistant-core appears to be running."
fi

echo "[hub-web] Launching Next.js dev server on http://localhost:3000"
(
  cd "$ROOT_DIR/web" &&
  if [ ! -f .env.local ]; then
    echo "NEXT_PUBLIC_API_BASE=$API_BASE" > .env.local
  fi &&
  # Ensure dependencies are installed
  if [ ! -x node_modules/.bin/next ]; then
    echo "[hub-web] Installing web dependencies..."
    if command -v pnpm >/dev/null 2>&1; then
      pnpm install
    elif command -v yarn >/dev/null 2>&1; then
      yarn install --frozen-lockfile || yarn install
    else
      if [ -f package-lock.json ]; then
        npm ci || npm install
      else
        npm install
      fi
    fi
  fi &&
  # Run dev server with the available package manager
  if command -v pnpm >/dev/null 2>&1; then
    pnpm dev --port 3000
  elif command -v yarn >/dev/null 2>&1; then
    yarn dev -p 3000
  else
    npm run dev
  fi
)
