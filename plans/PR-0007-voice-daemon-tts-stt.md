# PR-0007 — Voice Daemon (TTS/STT/VAD/Wake)

Summary: Implement the Python `voice-daemon` package with wake word, VAD, STT, and TTS engines (CosyVoice2/OpenVoice/Kokoro/Piper) via a streaming WS API.

Dependencies: PR-0001.

Deliverables

- `mcp-servers/python/voice_daemon/` with:
  - `pyproject.toml` and package `voice_daemon/` (server.py, router.py, schemas.py, audio.py, cache.py, ssml_lite.py).
  - `engines/` implementing base interface + cosyvoice2.py, openvoice.py, kokoro.py, piper.py.
  - Models dir `models/` (gitignored) managed by bootstrap script.
- WS endpoint `/v1/tts/stream` returning streaming PCM frames; `GET /v1/tts/health`.
- Config integration via `config/foreman.toml` [tts] section.

Wiring Checklist

- Add `audio_out` module in assistant-core and wire a WS client to the voice-daemon `/v1/tts/stream`.
- Expose `/api/voice/test` endpoint in core that triggers a short TTS stream and returns metrics.
- TUI `/voice test` command calls the core endpoint and shows status; add barge-in stop (space) to cancel playback.
- Respect license gating and engine fallbacks per `foreman.toml`.

Implementation Notes

- Standardize engine output to 24k mono PCM; resample in `audio.py`.
- Barge-in: global cancel flag set by core (later) or local hotkey; ensure stream stops promptly.
- Phrase cache for sub-200ms prompts.
- License gating: skip NC engines if `license_allow_nc=false`.

Tests

- Async pytest: synthesize short text via each engine (mocked if engines unavailable) → non-empty PCM; validate sample rate and RMS bounds.
- Health/bench endpoints return sensible metrics.

Acceptance Criteria

- `python -m voice_daemon` starts; `/v1/tts/health` reports readiness; `/v1/tts/stream` yields frames for a demo line.
- Core can play a test phrase through audio output; TUI `/voice test` works and barge-in cancels playback.

References

- docs/ARCHITECTURE.md (TTS Integration Plan), docs/TESTING.md

Out of Scope

- Core audio-out wiring (handled in a later core PR).
