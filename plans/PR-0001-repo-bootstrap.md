# PR-0001 — Repo Bootstrap & Scaffolding

Summary: Initialize the monorepo structure, Rust workspace, Python package layout, base configs, scripts, and minimal CI to build and test empty targets. No product features yet.

Dependencies: None.

Deliverables

- Directory skeleton matching `docs/REPO_LAYOUT.md` with placeholders.
- Root `Cargo.toml` workspace with empty members (buildable with stub crates/bins).
- `rust-toolchain.toml`, `.editorconfig`, `.gitignore`, `.env.example`.
- `Justfile` with: build, test, fmt, lint, run-core, run-tui.
- `scripts/` with `install-rust.sh`, `install-python.sh`, `bootstrap.sh`, `dev-run.sh` stubs.
- GitHub Actions workflows: `ci.yml` (fmt/clippy/build/test), `lint.yml` (ruff/black).

Directory Impact

- Create all top-level directories from `docs/REPO_LAYOUT.md` with README placeholders.
- Add minimal crates: `apps/assistant-core` and `apps/ui-tui` with `main.rs` printing version and exiting.
- Add placeholder Python packages: `mcp-servers/python/voice_daemon` and `arxiv_server` with `pyproject.toml` and empty `__main__.py`.

Implementation Notes

- Root `Cargo.toml`:
  - Set `resolver = "2"`; add the two app bins and at least one shared crate `foreman-types` with empty types.
- `Justfile` tasks (examples):
  - `build`: `cargo build --workspace`
  - `test`: `cargo test --workspace` and `pytest -q` under each Python package (guard if present).
  - `fmt`: `cargo fmt --all` and `black .` if Python packages exist.
- `.github/workflows/ci.yml`:
  - Cache cargo and pip; run fmt/clippy/build/test; tolerate missing Python servers initially.

Tests

- Rust: `cargo test --workspace` runs and passes with placeholder tests.
- Python: `pytest -q` discovers zero or trivial tests without error.

Acceptance Criteria

- Clean CI run on a clean checkout; no warnings promoted to errors.
- `just build`, `just test`, and `just fmt` succeed locally.

References

- docs/REPO_LAYOUT.md, docs/ARCHITECTURE.md, docs/TESTING.md

Out of Scope

- Any product logic (policy/memory/tools); add only stubs required to build.

