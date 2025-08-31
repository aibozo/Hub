# Realtime Voice-to-Voice (V2V)

This document describes the realtime V2V bridge that enables wake-driven, low‑latency speech-to-speech conversations using the OpenAI `gpt-realtime` model. It consolidates architecture, configuration, endpoints, testing, and wiring so no steps rely on tribal knowledge.

## Overview

- Transport: WebSocket initially; WebRTC optional in a follow-up (PR-0022).
  - Default transport is WebSocket. WebRTC is optional and feature-gated; if selected without support, the bridge returns a friendly error.
- Session: configured via `session.update` with `model: "gpt-realtime"`, modalities `["audio","text"]`, audio I/O formats, and V2V-specific instructions.
- Tools: all core tools are exposed to the model as JSON Schemas; requests go through the gatekeeper and approvals.
- Mode switch: wake phrase ("hey vim") or TUI command starts V2V; `end_call` tool ends the session and returns to T2T.

## Core Endpoints

- `POST /api/realtime/start` — start the realtime session; accepts overrides:
  - `{ model?, voice?, audio?: { in_sr?, out_format? }, instructions?, endpoint?, transport? }`
- `POST /api/realtime/stop` — stop the active session; idempotent.
- `GET /api/realtime/status` — `{ active, model, since, last_error? }`.

## Configuration

`config/foreman.toml` additions:

```
[voice]
wake_phrase = "hey vim"

[realtime]
# enabled defaults to false; feature-gated at build time
model = "gpt-realtime"
transport = "websocket"
endpoint = "wss://api.openai.com/v1/realtime"
audio_in_hz = 16000
audio_out_format = "g711_ulaw"
voice = "alloy"
```

Environment variables (optional overrides):
- `OPENAI_API_KEY`, `OPENAI_REALTIME_MODEL`, `OPENAI_REALTIME_ENDPOINT`.

## Build & Features

- Feature flag: `realtime` (enables the WS client and tests), `realtime-audio` (audio I/O: capture + playback).
- Examples:
  - `cargo test -p assistant-core --features realtime`
  - `cargo run -p assistant-core` (core) and `cargo run -p assistant-core --bin rt-probe -- status` (dev harness)

### Audio I/O

- When built with `--features realtime-audio`, the bridge:
  - Captures mic PCM16 frames and sends `input_audio_buffer.append` events (semantic VAD may auto‑commit turns).
  - Decodes `g711_ulaw` (or `pcm16`) output frames and plays them via the default output device with a jitter buffer.

## Dev Harness: `rt-probe`

- `rt-probe status` — show `/api/realtime/status`.
- `rt-probe start [endpoint]` — start a session (override endpoint for mocks).
- `rt-probe stop` — stop the active session.

## Tool Calling

- Schemas are generated from `config/tools.d/*.json` and injected into `session.update`.
- All tool requests are evaluated by the gatekeeper and approvals flow before execution.
- Synthetic tool `end_call` terminates the session cleanly.

### Approvals Flow (Realtime)

- The bridge evaluates each `tool.call` against policy. If a call is not allowed outright, it raises an ephemeral approval prompt (`/api/approval/prompt`) containing the proposed action and arguments.
- The TUI can surface this prompt; answering it via `/api/approval/answer` clears the prompt and the bridge retries the tool call.
- If no answer is received within a timeout (default 120s), the bridge returns a `tool.output` error indicating an approval timeout.

## Testing

- Realtime tests are no-network by default and run against a local mock WS server.
- Run: `cargo test -p assistant-core --features realtime`.
- See `apps/assistant-core/tests/realtime_mock.rs` for example coverage of connect/start/stop.

## Follow-Up PRs & Wiring

- PR-0017: Tool bridge & `end_call` — integrate tool schemas and bridge gatekeeper/tool execution.
- PR-0018: Audio I/O — mic capture and audio playback with jitter buffer.
- PR-0019: Wake sentinel — VAD + wake phrase detection to start/stop sessions (`/api/wake/{status,enable,disable}`), uses energy-based VAD with STT-based phrase match.
  - If `OPENAI_API_KEY` is set, the sentinel uses OpenAI Whisper (HTTP) to transcribe short speech segments (WAV in-memory) for wake phrase detection. In tests/CI, this path is not exercised.
- PR-0020: TUI wiring — status, controls, help overlay.
- PR-0021: History sharing — seed realtime with chat turns; append summaries on end.
- PR-0022: WebRTC transport — optional lower-latency path.
  - Config: `[realtime] transport = "webrtc"` or pass `transport: "webrtc"` in `/api/realtime/start`.
  - Current build defaults to WebSocket; WebRTC is stubbed unless compiled with the appropriate feature.

Track status and wiring in `docs/WIRING_MATRIX.md`. Each follow-up PR carries a Wiring Checklist and updates the matrix. The initial bridge lands in PR-0023.
