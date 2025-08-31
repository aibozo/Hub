# PR-0009 — Schedulers & Daily Briefs

Summary: Implement a cron-like scheduler in core and wire two timed jobs: arXiv sweep and News risk brief, writing cached summaries as artifacts.

Dependencies: PR-0002, PR-0004 (artifacts), PR-0010 (arXiv/news servers).

Deliverables

- `apps/assistant-core/src/scheduler.rs`: cron parser, timezone `America/Indiana/Indianapolis`, job registration, graceful error handling.
- Config `config/schedules.toml` with defaults: arxiv at 07:30, news at 08:00.
- Job implementations call into MCP servers to fetch summaries and write artifacts + events.

Wiring Checklist

- Load schedules from `config/schedules.toml`; expose `/api/schedules` for introspection.
- Wire arXiv/news MCP servers (PR-0010) into the scheduled jobs with retries/backoff.
- Write brief markdown artifacts and append memory atoms summarizing the brief (1–2 sentences + link).
- TUI displays upcoming jobs and last run status in Tasks or a Briefs pane.

Implementation Notes

- Use a single scheduler task with tokio intervals; compute next tick per job respecting TZ.
- Artifacts named with date prefixes; linkable via `artifact://` URIs in memory/events.

Tests

- Unit: next-run computations across DST boundaries.
- Integration: with fake clock, trigger both jobs and assert artifact creation.

Acceptance Criteria

- Jobs run at configured times; on failure, errors are logged and retried with backoff.
- `/api/schedules` reflects job states; artifacts and memory atoms are created; TUI shows last-run summaries.

References

- docs/ARCHITECTURE.md (Schedulers), docs/TOOLS.md (arXiv/news)

Out of Scope

- TUI display of briefs (added in a later UI PR).
