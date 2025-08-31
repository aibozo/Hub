# PR-0019 — Wake Sentinel (VAD + Wake Phrase)

Summary: Add an always-on wake sentinel that listens to the microphone, detects the wake phrase ("hey vim"), and triggers the realtime session start. Manage streaming lifecycle and debounce false positives.

Dependencies: PR-0023 (realtime bridge), PR-0018 (audio I/O)

## Deliverables
- Wake sentinel service (Rust task in core or Python module co-located with voice daemon; choose Rust for tighter integration and fewer deps).
- VAD: semantic or energy-based, configurable sensitivity; fallback to `webrtcvad` if available.
- Wake phrase detection: initial STT-based detection on buffered audio (Whisper API or local engine if configured); fuzzy matching for "hey vim".
- Control: calls `/api/realtime/start` on detection; streams mic frames (via PR-0018); stops on `end_call` or inactivity.
- Commands: `/voice enable_wake`, `/voice disable_wake` via TUI (wired in PR-0020).
- Config: `[voice] wake_phrase`, `wake_enabled`, `vad_sensitivity`, `min_speech_ms`, `max_turn_ms`.

## Implementation
- Module: `apps/assistant-core/src/wake.rs` (tokio task) or `mcp-servers/python/voice_daemon/wake_sentinel.py` (if Python tools needed). Prefer Rust for determinism.
- Buffer recent audio (e.g., 2–3s ring buffer). On VAD speech onset, collect phrase window and run STT (gated by `OPENAI_API_KEY` or local engine in config), compare to wake phrase.
- Debounce: refractory period (e.g., 3–5s) after activation; configurable.
- Logging: structured events to memory and `storage/logs/assistant-core.log`.

## Tests
- Unit: fuzzy match thresholds; VAD state transitions on synthetic envelopes.
- Integration: feed PCM fixtures with and without wake phrase; assert activation only on positives; ensure deactivation after inactivity.

## Acceptance Criteria
- Wake phrase reliably triggers realtime start in quiet/typical environments.
- False positives are below acceptable threshold with default sensitivity; can be tuned via config.
- Disabling wake stops the sentinel promptly.

## Wiring Checklist
- `[voice]` config consumed; toggles exposed; status reported via `/api/realtime/status`.
- Sentinel cooperates with realtime manager; no double-starts; clean shutdown.
