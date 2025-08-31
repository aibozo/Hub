# PR-0022 — Realtime WebRTC Transport (Optional)

Summary: Add WebRTC transport support for the realtime bridge for lower latency and NAT traversal, as an alternative to WebSocket. Feature-gated and off by default.

Dependencies: PR-0023 (realtime bridge), PR-0018 (audio I/O)

## Deliverables
- WebRTC client integration (Rust), session offer/answer handling per docs.
- Config: `[realtime] transport = "webrtc"` switches transport layer.
- Fallback to WS on negotiation failure; clear errors surfaced in status.

## Implementation
- Abstraction in `realtime.rs` over Transport: `WsTransport` and `RtcTransport` implementing a common trait.
- Signaling: use OpenAI endpoint for SDP exchange; route ICE candidates.
- Audio: keep the same input/output PCM pipelines; map to WebRTC audio tracks.

## Tests
- Unit: SDP parse/build; transport selection logic.
- Integration: mock signaling path; ensure event streams are routed equivalently to WS.

## Acceptance Criteria
- With `transport=webrtc`, session connects and audio flows comparably to WS.
- Fallback to WS works when WebRTC not available.

## Wiring Checklist
- Transport toggle documented and feature-gated.
- Logs & status reflect transport selection and failures.
