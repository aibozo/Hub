#!/usr/bin/env bash
set -euo pipefail

# Simple launcher: starts assistant-core in background (if not already up),
# waits for readiness, runs the TUI, then stops the core on exit.

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")"/.. && pwd)"
cd "$ROOT_DIR"

: "${FOREMAN_ENV_FILE:=.env}"
# Load environment from .env if present (API keys, etc.)
if [ -f "$FOREMAN_ENV_FILE" ]; then
  # export all vars in the file (simple KEY=VALUE lines)
  set -a
  # shellcheck disable=SC1090
  source "$FOREMAN_ENV_FILE"
  set +a
fi

: "${FOREMAN_BIND:=127.0.0.1:6061}"
READY_URL="http://${FOREMAN_BIND}/ready"

: "${VOICE_HOST:=127.0.0.1}"
: "${VOICE_PORT:=7071}"
VOICE_BASE="http://${VOICE_HOST}:${VOICE_PORT}"
VOICE_READY_URL="${VOICE_BASE}/v1/tts/health"

log() { echo "[tui-run] $*"; }

have_curl=0
if command -v curl >/dev/null 2>&1; then have_curl=1; fi

check_ready() {
  if [ $have_curl -eq 1 ]; then
    curl -fsS --max-time 1 "$READY_URL" >/dev/null 2>&1 && return 0 || return 1
  else
    # Fallback using bash/tcp (best effort)
    : >/dev/tcp/"${FOREMAN_BIND%:*}"/"${FOREMAN_BIND#*:}" && return 0 || return 1
  fi
}

ensure_logs_dir() {
  mkdir -p storage/logs || true
}

CORE_PID=""
OWNED_CORE=0
VOICE_PID=""
OWNED_VOICE=0

cleanup() {
  local code=$?
  if [ "$OWNED_VOICE" -eq 1 ] && [ -n "$VOICE_PID" ] && kill -0 "$VOICE_PID" >/dev/null 2>&1; then
    log "Stopping voice-daemon (pid=$VOICE_PID)"
    kill "$VOICE_PID" 2>/dev/null || true
    sleep 0.1 || true
  fi
  if [ "$OWNED_CORE" -eq 1 ] && [ -n "$CORE_PID" ] && kill -0 "$CORE_PID" >/dev/null 2>&1; then
    log "Stopping assistant-core (pid=$CORE_PID)"
    # Try graceful SIGINT first (tokio::signal::ctrl_c)
    kill -INT "$CORE_PID" 2>/dev/null || true
    for _ in {1..20}; do
      kill -0 "$CORE_PID" >/dev/null 2>&1 || break
      sleep 0.1
    done
    if kill -0 "$CORE_PID" >/dev/null 2>&1; then
      kill "$CORE_PID" 2>/dev/null || true
      sleep 0.1 || true
    fi
  fi
  exit $code
}
trap cleanup EXIT INT TERM

if check_ready; then
  log "assistant-core already running at $FOREMAN_BIND"
else
  ensure_logs_dir
  TS="$(date +%Y%m%d-%H%M%S)"
  LOG_FILE="storage/logs/assistant-core-${TS}.log"
  log "Starting assistant-core at $FOREMAN_BIND (logs: $LOG_FILE)"
  # Run in background; inherit environment. Optional features via CORE_FEATURES env (comma-separated)
  # Enable realtime + audio features by default; override via CORE_FEATURES
  CORE_FEATURES_VAL="${CORE_FEATURES:-realtime,realtime-audio}"
  ( RUST_LOG=${RUST_LOG:-info} FOREMAN_BIND="$FOREMAN_BIND" cargo run -p assistant-core --bin assistant-core --features "$CORE_FEATURES_VAL" ) \
    >>"$LOG_FILE" 2>&1 &
  CORE_PID=$!
  OWNED_CORE=1

  # Wait for readiness (up to ~60s) and fail fast if the process exits early
  for i in {1..120}; do
    if check_ready; then break; fi
    if ! kill -0 "$CORE_PID" >/dev/null 2>&1; then
      log "Error: assistant-core exited before readiness (pid=$CORE_PID)"
      log "--- tail of core log ---"
      tail -n 200 "$LOG_FILE" || true
      exit 1
    fi
    sleep 0.5
  done
  if ! check_ready; then
    log "Error: assistant-core did not become ready at $READY_URL within timeout"
    log "--- tail of core log ---"
    tail -n 200 "$LOG_FILE" || true
    exit 1
  fi
fi

check_voice_ready() {
  if [ $have_curl -eq 1 ]; then
    curl -fsS --max-time 1 "$VOICE_READY_URL" >/dev/null 2>&1 && return 0 || return 1
  else
    : >/dev/tcp/"${VOICE_HOST}"/"${VOICE_PORT}" && return 0 || return 1
  fi
}

# Start voice-daemon if not running
if [ -n "${PICOVOICE_ACCESS_KEY:-}" ]; then
  log "Porcupine: PICOVOICE_ACCESS_KEY present"
else
  log "Porcupine: PICOVOICE_ACCESS_KEY missing (set it in .env)"
fi

if check_voice_ready; then
  log "voice-daemon already running at ${VOICE_BASE}"
else
  ensure_logs_dir
  TS="$(date +%Y%m%d-%H%M%S)"
  VLOG_FILE="storage/logs/voice-daemon-${TS}.log"
  if command -v python >/dev/null 2>&1; then
    log "Starting voice-daemon at ${VOICE_BASE} (logs: $VLOG_FILE)"
    ( VOICE_HOST="$VOICE_HOST" VOICE_PORT="$VOICE_PORT" python -m voice_daemon ) >>"$VLOG_FILE" 2>&1 &
    VOICE_PID=$!
    OWNED_VOICE=1
    for i in {1..60}; do
      if check_voice_ready; then break; fi
      sleep 0.5
    done
    if ! check_voice_ready; then
      log "Warning: voice-daemon did not become ready at $VOICE_READY_URL; wake STT may be unavailable"
    fi
  else
    log "Warning: Python not found; cannot start voice-daemon"
  fi
fi

log "Launching TUI (ui-tui)"
if ! command -v ffmpeg >/dev/null 2>&1 && ! command -v arecord >/dev/null 2>&1; then
  log "Warning: neither 'ffmpeg' nor 'arecord' found; voice PTT will not work"
fi
RUST_LOG=${RUST_LOG:-info} cargo run -p ui-tui --features tui,http,voice
