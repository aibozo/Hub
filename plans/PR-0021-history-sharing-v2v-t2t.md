# PR-0021 — History Sharing Between V2V and T2T

Summary: Share conversation context between realtime V2V sessions and standard text chat. Seed the realtime session with recent chat turns, and on `end_call`, append a compact summary of the V2V segment back into the chat history.

Dependencies: PR-0023 (realtime bridge), PR-0020 (TUI wiring)

## Deliverables
- History seeding: before `session.update`, compile recent N turns (user/assistant), compress to a compact digest (no long artifacts), and include as text inputs.
- End-of-call summary: on `end_call`, produce an assistant-visible summary (tools invoked, outcomes, decisions) and append to chat session.
- Memory/Events: log start/stop + tool decisions as events; optionally store audio artifact references.

## Implementation
- Core `realtime.rs`: add helpers to fetch latest chat session, select turns (token budget aware), and inject as initial text inputs.
- On stop, create a summarized message with links to artifacts/events; append via existing chat session APIs.
- Ensure alignment with context packer rules (avoid sensitive or huge payloads).

## Tests
- Unit: turn selection heuristics; summary formatting bounded by length.
- Integration: mock a chat session with prior turns; ensure seeding inserts; end call writes a summary message.

## Acceptance Criteria
- Realtime replies reflect prior context; after ending, the chat contains a concise summary of the voice segment.

## Wiring Checklist
- No duplication of messages; summaries are clearly marked and compact.
- Works with in-memory DB (tests) and file-backed DB (manual).
