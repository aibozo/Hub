# PR-0200 — Web UI Enablement: CORS & Policy

Summary: Enable safe CORS in assistant-core so a local Next.js app can call the HTTP and POST‑SSE endpoints. Document the Web UI addition and update wiring.

Dependencies: PR-0002 (assistant-core skeleton), PR-0003 (policy/gatekeeper).

Deliverables

- Add a `tower_http::cors::CorsLayer` to the Axum router in `apps/assistant-core/src/main.rs` (or layered in `api::build_router`), allowing localhost origins.
  - Origins: `http://127.0.0.1:3000`, `http://localhost:3000` (dev). Optionally `Any` in dev builds; keep tight for release.
  - Methods: `GET, POST, DELETE, OPTIONS`.
  - Headers: `Content-Type, Authorization`.
  - Expose headers: optional `x-sse-event` if used.
  - Cache preflight: `max_age = 86400`.
- Ensure `POST /api/chat/stream` works cross‑origin (CORS preflight passes, `Content-Type: application/json` allowed).
- Add docs:
  - `docs/WEBUI_SPEC.md` (this spec, index link).
  - Update `docs/ARCHITECTURE.md` and `docs/REPO_LAYOUT.md` to note the planned `web/` app and shared core API.
- Update `docs/WIRING_MATRIX.md` with a new Web UI section (planned PRs).

Wiring Checklist

- CORS enabled for JSON and `text/event-stream` responses.
- No bypass of gatekeeper: Web UI calls only the existing HTTP API.
- Preflight OPTIONS returns appropriate `Access-Control-Allow-*` headers.

Tests

- Add an integration test for CORS preflight on `/api/chat/stream` and a simple JSON GET (`/health`), verifying `Access-Control-Allow-Origin` and allowed methods/headers.
- Run `cargo test --workspace` locally.

Acceptance Criteria

- Browser app (running on localhost:3000) can call `/ready`, `/api/chat/stream` (POST+SSE), and `/api/chat/sessions*` without CORS errors.
- Policy docs explicitly state Web UI is a consumer of the same core API and does not add new execution surfaces.

Out of Scope

- Web UI code (added in PR‑0201 and later).

Rollback Plan

- Remove the CorsLayer and doc references; no data migration involved.

References

- docs/WEBUI_SPEC.md
- docs/POLICY.md
- apps/assistant-core/src/main.rs, apps/assistant-core/src/api.rs

