# PR-0023 — Realtime Bridge (V2V) & Wakeword Activation

Summary: Introduce a low‑latency voice‑to‑voice (V2V) mode gated by a wake phrase ("hey vim"), backed by OpenAI Realtime model `gpt-realtime` over WebSocket. Provide a core bridge that: (1) establishes and manages the realtime session, (2) streams mic audio in and TTS audio out, (3) exposes and enforces our existing tool‑calling via the gatekeeper, (4) cleanly transitions between V2V and text‑to‑text (T2T) modes with an explicit `end_call` tool. Include a dev harness to iterate on connection and tool‑calling independent of the TUI.

Dependencies: PR-0007 (voice-daemon TTS skeleton), PR-0008 (TUI skeleton)


## Scope

This PR focuses on the core realtime bridge, API endpoints, tool bridging, and an external dev harness. Audio I/O (mic capture/playback) and wakeword detection are partially stubbed and will be fully wired in follow‑ups with explicit IDs below.

In Scope (implemented here):
- Realtime WebSocket client in core (`apps/assistant-core`) with session lifecycle management.
- Session configuration for `model: "gpt-realtime"`, audio/text modalities, and v2v instructions.
- Tool‑calling bridge: realtime tool requests → gatekeeper → tools → realtime `tool.output`.
- `end_call` virtual tool to terminate the V2V session and return to T2T.
- Core REST endpoints to start/stop/status realtime sessions.
- Dev harness CLI (`rt-probe`) to test connection, session updates, tool calls, and text responses without TUI.
- Deterministic unit/integration tests using a local mock realtime server (no network).

Out of Scope (follow‑ups):
- PR-0018 — Audio I/O playback & mic streaming (low‑latency PCM path in Rust; jitter buffer).
- PR-0019 — Wakeword sentinel (VAD + wake phrase; triggers realtime start and streams mic frames).
- PR-0020 — TUI wiring (status indicators, hang‑up hotkey, help overlay, controls).
- PR-0021 — Conversation sharing between V2V and T2T (seeding realtime from chat history; appending V2V turns back).
- PR-0022 — WebRTC transport option (optional; start with WebSocket).

All out‑of‑scope items are explicitly linked (IDs above) and must be added to `docs/WIRING_MATRIX.md` during their PRs per Wiring Policy.


## Architecture & Flows

Components:
- Realtime Bridge (core): `apps/assistant-core/src/realtime.rs`
  - Manages a WS connection to `wss://api.openai.com/v1/realtime`.
  - Sends `session.update` with model, modalities, audio config, tools, and v2v instructions.
  - Sends/receives structured events (`response.create`, `response.*`, `tool.*`, `input_audio_buffer.*`).
  - Bridges tool calls through gatekeeper and tools manager; enforces policies/approvals.
  - Exposes REST control endpoints in `apps/assistant-core/src/api.rs`.
- Tool Bridge:
  - Translates realtime tool schema ↔ our `config/tools.d/*.json` manifests.
  - Executes tools via core Tools Manager (stdio MCP servers or in‑core tools) under gatekeeper.
  - Returns `tool.output` with JSON results; emits provenance/approvals as usual.
- Audio I/O (follow‑up PR-0018):
  - Mic → `input_audio_buffer.append` frames; commit on VAD turn boundaries.
  - Model audio output → decode (e.g., G.711 µ‑law to PCM) → play via `cpal`/`rodio`.
- Wake Sentinel (follow‑up PR-0019):
  - Always‑on mic watcher with VAD and wake phrase detection (initial STT‑based wake; configurable phrase `"hey vim"`).
  - On wake: call `/api/realtime/start` and begin streaming frames.
  - On `end_call` tool or inactivity: stop and revert to idle.

State Machine (high level):
- T2T idle → on wake phrase → V2V Starting → V2V Active → (user says “end call” or model calls `end_call`) → V2V Stopping → T2T idle.
- Errors lead to V2V Aborted → T2T idle with user‑visible toast + logs.


## Session Configuration (session.update)

- Model: `"gpt-realtime"` (exact name).
- Modalities: `["audio","text"]`.
- Audio:
  - `input.format: "pcm16"` (mono, 16 kHz or 24 kHz as configured), `turn_detection: { type: "semantic_vad", create_response: true }`.
  - `output.format: "g711_ulaw"` (low‑latency; configurable) and `voice` (e.g., "alloy").
- Prompting/Instructions (V2V‑only):
  - “You are in Voice‑to‑Voice mode. Speak concisely. After each tool run, briefly summarize the result, then ask: ‘Anything else?’. If the user declines (e.g. ‘no thanks that’ll be all’), call the tool `end_call` to terminate this voice session and hand control back to text chat. Use the provided tools (with arguments) and wait for results before continuing. Respect approvals and policies.”
- Tools: include JSON‑Schema for all core tools and the `end_call` virtual tool.

Example (abridged) `session.update` payload:
```json
{
  "type": "session.update",
  "session": {
    "model": "gpt-realtime",
    "modalities": ["audio","text"],
    "audio": {
      "input": { "format": "pcm16", "turn_detection": { "type": "semantic_vad", "create_response": true } },
      "output": { "format": "g711_ulaw", "voice": "alloy", "speed": 1.0 }
    },
    "instructions": "You are in Voice-to-Voice mode...",
    "tools": [ /* generated schemas incl. end_call */ ]
  }
}
```


## Event Mapping

Client → Server:
- `session.update` (configure model, modalities, audio, instructions, tools).
- `input_audio_buffer.append` (binary PCM16 audio chunks).
- `input_audio_buffer.commit` (end of user turn if manual; otherwise rely on semantic VAD when enabled).
- `response.create` (optional nudge to produce a response, if not auto‑created by VAD).
- `tool.output` (after executing a requested tool and receiving approval if required).

Server → Client:
- `session.updated` (ack of session settings).
- `response.*` events (delta, completed, audio output frames, transcripts).
- `tool.call` (tool name + JSON args requested by the model).

Error Handling:
- Map transport/HTTP errors to specific, retryable vs terminal status.
- On authentication error: fail fast, surface to `/api/realtime/status` and logs.
- On tool bridge error: respond with a `tool.output` containing an error object and prompt the model to ask for clarification.


## Tool‑Calling Bridge

Schema Generation:
- Derive JSON Schemas from `config/tools.d/*.json` manifests (tool name, params schema) to present to the realtime session.
- Inject a synthetic tool:
  - `end_call`: `{ name: "end_call", description: "End the voice session and return to text chat.", parameters: { "type": "object", "properties": {}, "additionalProperties": false } }`.

Execution Flow:
- On `tool.call`:
  1. Construct `ProposedAction` with tool name/args.
  2. Gatekeeper evaluates policy; if approval required, create ephemeral prompt and pause.
  3. On approval, call tool via Tools Manager; capture provenance, events, and artifacts.
  4. Send `tool.output` with JSON result (or error object) back to realtime session.
- On `end_call`:
  - Stop streaming audio, close WS gracefully, mark mode to T2T.
  - Append a short text summary of the V2V segment to the shared chat history (PR-0021).


## Core API (assistant‑core)

New endpoints:
- `POST /api/realtime/start` → starts a WS session if none active; accepts options:
  - `{ model?: string, voice?: string, audio?: { in_sr?: 16000|24000, out_format?: "g711_ulaw"|"pcm16" }, instructions?: string }`
- `POST /api/realtime/stop` → graceful shutdown if active (also invoked by `end_call`).
- `GET /api/realtime/status` → `{ active: bool, model: string|null, since: string|null, last_error?: string }`.

Constraints:
- Single active realtime session per process.
- Reject `start` if already active with 409; `stop` is idempotent.


## Config & Features

`config/foreman.toml` additions:
```toml
[voice]
wake_phrase = "hey vim"             # previously "hey foreman"

[realtime]
enabled = false
model = "gpt-realtime"
transport = "websocket"             # or "webrtc" (PR-0022)
endpoint = "wss://api.openai.com/v1/realtime"
audio_in_hz = 16000                  # or 24000
audio_out_format = "g711_ulaw"       # or "pcm16"
voice = "alloy"
```

Env:
- `OPENAI_API_KEY` (required for real endpoint; not needed for mocks/tests).
- `OPENAI_REALTIME_ENDPOINT` (optional override).
- `OPENAI_REALTIME_MODEL` (optional override).

Cargo features:
- `realtime` (guards the WS bridge & rt‑probe binary).
- `realtime-audio` (guards audio I/O, added in PR-0018).


## Implementation Details

- Module: `apps/assistant-core/src/realtime.rs`
  - `RealtimeManager` (Arc): holds connection state, tool schemas, and a task handle.
  - Public API:
    - `start(opts) -> Result<()>`
    - `stop() -> Result<()>`
    - `status() -> RealtimeStatus`
  - Internals:
    - Build `session.update` with instructions, tools, audio config.
    - Event loop task: `tokio::select!` on WS messages and control signals.
    - Tool bridge handler: routes `tool.call` → gatekeeper/tools → `tool.output`.
    - Metrics: counters for `realtime_sessions_started`, `realtime_sessions_errors`, `tool_calls`, latencies.
    - Logs: write details to `storage/logs/assistant-core.log`.

- API wiring: update `apps/assistant-core/src/api.rs` with 3 endpoints calling `RealtimeManager`.
- Tools schemas: helper `tools::to_json_schemas()` to emit schema list + `end_call`.
- Audio I/O stubs: accept optional PCM payloads from an external tester (rt‑probe) and forward to WS (`input_audio_buffer.append`), gated by feature flag until PR-0018.


## Dev Harness: `rt-probe`

Purpose: Iterate on WS connection, session updates, tool calls, and event pump without TUI; switchable between mock server and real API.

CLI modes:
- `rt-probe connect` → open WS, send `session.update`, print `session.updated`.
- `rt-probe say --text "hello"` → send a text `response.create` and print response deltas.
- `rt-probe tool --name shell.exec --json '{"cmd":"echo","args":["hi"]}'` → simulate tool call flow (via a mock server in tests; for real API, model must request the tool).
- `rt-probe audio --wav samples/hello.wav` → stream PCM16 frames to `input_audio_buffer` (feature‑gated until PR-0018).
- `rt-probe end` → invoke `end_call` (locally or by sending a tool result for `end_call`).

Config flags:
- `--endpoint`, `--model`, `--voice`, `--in-hz`, `--out-format`.


## Tests

Unit (no network):
- Tools schema generation produces expected JSON (includes `end_call`, correct params for each manifest).
- Realtime manager builds correct `session.update` JSON from config.
- Tool bridge: simulated `tool.call` → gatekeeper mock → returns `tool.output` JSON with result or error.

Integration (mock server):
- Spawn a local WS server emulating minimal realtime flows:
  - Accept `session.update`, echo `session.updated`.
  - After a `response.create`, push a small text delta then complete.
  - Issue a synthetic `tool.call`, expect `tool.output` with specific result.
  - On `end_call`, ensure manager stops and `status.active = false`.
- `rt-probe` connects to mock; asserts event timeline and exit codes.

No‑network CI: All tests rely on the mock server; real endpoint is feature‑gated and skipped in CI.

Manual (opt‑in):
- With `OPENAI_API_KEY`, run `rt-probe connect` and `rt-probe say` against the real endpoint to validate auth/handshake.


## Policy & Safety

- All tool executions originate from realtime `tool.call` events but are routed through the existing gatekeeper (policy checks, approvals, provenance).
- `end_call` is non‑privileged but must be auditable (logged as a provenance event).
- No file writes outside `storage/`; audio buffers are transient.
- Timeouts and retry backoffs for WS connection and tool calls.


## Telemetry

- Counters: `realtime_sessions_started`, `realtime_sessions_active`, `realtime_sessions_errors`, `realtime_tool_calls_total`.
- Histograms: WS connect time, tool call latency, audio frame enqueue/dequeue times (PR-0018).


## Acceptance Criteria

- `/api/realtime/start` starts a WS session (mocked in tests) and `status.active=true`.
- Tool schemas are published in `session.update`; mock server issues `tool.call` and receives valid `tool.output`.
- `end_call` cleanly stops the session; `status.active=false`.
- `rt-probe` can connect to mock, perform `say`, handle a mocked tool call, and end call.
- No network required for tests; CI passes deterministically.


## Rollout & Migration

- Default `enabled=false`; feature flag `realtime` off by default.
- After this PR lands, proceed with PR-0018 (audio), PR-0019 (wake), PR-0020 (TUI), PR-0021 (history), PR-0022 (WebRTC) with small, reviewable changes.


## Risks & Mitigations

- Transport drift: mock to stabilize event contracts; guard real endpoint under feature flag.
- Tool schema drift: generate from single source (tool manifests) to avoid duplication.
- Latency/audio glitches (future PR): start with µ‑law and minimal jitter buffer; measure and iterate.
- Wake false positives (future PR): begin with STT‑based wake on buffered audio; offer `/voice disable_wake`.


## Wiring Checklist

- Realtime manager module present; `start/stop/status` endpoints exposed.
- Tool schemas generated and include `end_call`.
- Tool bridge enforces gatekeeper checks and approvals path.
- `rt-probe` binary available and documented.
- Tests: unit + mock integration; no network used.
- Follow‑ups linked: PR-0018 (audio), PR-0019 (wake), PR-0020 (TUI), PR-0021 (history), PR-0022 (WebRTC).
- Update `docs/WIRING_MATRIX.md` in each follow‑up PR for any stubbed items.


## References

- Realtime docs: `docs/Realtime conversations - OpenAI API.html` (local copy).
- Existing code:
  - TUI PTT: `apps/ui-tui/src/audio.rs`, `apps/ui-tui/src/app.rs`, `apps/ui-tui/src/keymap.rs`.
  - Voice daemon TTS skeleton: `mcp-servers/python/voice_daemon/*`.
  - Config example: `config/foreman.toml`.
