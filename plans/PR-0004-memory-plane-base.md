# PR-0004 — Memory Plane Base (SQLite + Events + Atoms)

Summary: Establish the memory layer schema and adapters: SQLite migrations, event log, atoms store, and basic query APIs. No BM25/vector indexing yet.

Dependencies: PR-0002.

Deliverables

- `apps/assistant-core/migrations/` with initial schema for Task, TaskDigest, Atom, Artifact, Event.
- `apps/assistant-core/src/memory/`:
  - `mod.rs`, `schema.rs`, `events.rs`, `atoms.rs`, `context_pack.rs` (stub), `indices.rs` (stub).
- `crates/foreman-memory/`: re-usable adapters for SQLite connections and basic operations.

Wiring Checklist

- Wire event logging in assistant-core: every API request and MCP call appends an Event.
- Implement Tasks API: `POST /api/tasks`, `GET /api/tasks`, `GET /api/tasks/{id}` using SQLite.
- Expose context pack endpoint: `POST /api/context/pack` that returns a pack using stubbed indices.
- TUI Memory screen (PR-0008) reads from these endpoints without code shape changes later.

Implementation Notes

- Use `sqlx` with offline mode and migrations.
- Provide APIs: append_event, put_atom, get_atoms_by_task, upsert_task_digest.
- Use `storage/sqlite.db` by default (configurable path).

Tests

- Unit: migrations apply; CRUD operations on atoms/events; task digest upsert.
- Integration: create a task, append events/atoms, compute a trivial context pack (stub) under budget.

Acceptance Criteria

- Core can start, apply migrations on first run, and perform basic memory operations.
- Tasks API and event logging are live; context pack endpoint returns a pack under a budget.

References

- docs/MEMORY.md, docs/ARCHITECTURE.md (Memory section), docs/TESTING.md

Out of Scope

- Tantivy/HNSW indices (added in a follow-up PR).
