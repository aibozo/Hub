# PR-0018 — Realtime Audio I/O (Low-Latency)

Summary: Implement low-latency audio input/output for the realtime bridge. Stream mic PCM16 frames into the `gpt-realtime` session and play model audio output locally with minimal jitter.

Dependencies: PR-0023 (realtime bridge)

## Deliverables
- `realtime-audio` feature in core enabling audio capture/playback.
- Input: PCM16 mono capture at 16k/24k, chunked (e.g., 20–40 ms) to `input_audio_buffer.append`.
- Output: decode `g711_ulaw` (or `pcm16`) frames from server, optional small jitter buffer, playback via `cpal`/`rodio`.
- Config options in `[realtime]`: `audio_in_hz`, `audio_out_format`, `output_device` (optional).
- Metrics: output underruns/overruns, avg latency.

## Implementation
- Module: `apps/assistant-core/src/realtime_audio.rs` guarded by `realtime-audio`.
- Capture: use `cpal` input stream; convert to PCM16; enqueue to a bounded channel consumed by realtime bridge.
- Playback: `rodio` or raw `cpal` output stream; decode µ-law to PCM on the fly; apply 50–100 ms jitter buffer.
- Backpressure: drop policy on input if bridge backlogged; log counters.

## Tests
- Unit: µ-law decode correctness; jitter buffer enqueue/dequeue consistency.
- Integration (no hardware): simulate input frames via a generator; route through bridge to mock server; assert `append` cadence and commit boundaries.
- Manual: device listing, latency sanity via `rt-probe audio`.

## Acceptance Criteria
- Audio output is continuous with no significant stutter on typical hardware.
- Input frames flow to the server at expected cadence; server responses are audible.
- Metrics expose underruns/latency; no panics on device errors.

## Wiring Checklist
- `realtime-audio` feature flag documented.
- `[realtime]` config respected.
- Metrics registered and visible in `/metrics`.
- No blocking I/O in UI; logs to `storage/logs/assistant-core.log`.
