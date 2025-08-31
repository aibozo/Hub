Voice Daemon (TTS/STT)
======================

Lightweight local voice server exposing:
- GET /v1/tts/health → readiness metrics
- WS /v1/tts/stream → streaming PCM frames (24kHz mono)

Quick start:
- With optional deps: `pip install -e .[servers]`
- Run: `python -m voice_daemon` (health on 7071; WS on 7071 if aiohttp/websockets present, else fallback prints instructions)

Notes:
- Engines are pluggable; defaults to a NullTTS sine-wave generator for demos.
- Models live under `mcp-servers/python/voice_daemon/models/` (gitignored).

