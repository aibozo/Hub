# Hub Web UI — Product & Implementation Spec

Audience: engineers/designers building the local web frontend

Goals:

- Preserve all existing agent/codex/chat functionality.
- Deliver a ChatGPT‑style dark interface with tabs for Chat / Agents / Research.
- Keep UX minimal and ergonomic: defaults first, advanced controls tucked away.
- Support localhost usage against the existing core API (HTTP + POST‑SSE).
- Provide a modular theming system (default dark grey + teal accent; easily swappable).

This document is the source‑of‑truth spec for a sleek, minimal, dark‑grey/teal local web UI for the Hub. It covers aesthetics, component APIs, page layouts, data flows, accessibility, and acceptance criteria so contributors can build without guesswork.

## 1) Guiding Principles

1. Minimal surface area. Prioritize defaults and sensible presets.
2. One‑screen focus. Chat, Agents, and Research each get a dedicated workspace with a small contextual sidebar.
3. Progressive disclosure. Advanced settings live in drawers/collapsibles; avoid settings sprawl.
4. Readable at a glance. Tight rhythm; avoid visual noise and heavy dividers.
5. Keyboard‑centric. Full navigation + Command Palette.
6. Accessible by default. High contrast, visible focus rings, SR labels, reduced‑motion variants.
7. Stateless empathy. Show empty states, loading, and recovery paths.

## 2) Tech Stack (frontend)

- Next.js (App Router) + TypeScript
- Tailwind CSS + shadcn/ui
- React Query (server cache) + Zustand (global UI state)
- POST‑SSE streaming via `fetch()` + `ReadableStream`
- Monaco editor (code blocks, diffs)
- Lucide icons

Project layout:

```
web/
  app/
    layout.tsx
    page.tsx              # Chat (default)
    agents/page.tsx
    research/page.tsx
    settings/page.tsx     # optional; or a modal
  components/
    app/TopNav.tsx
    app/Sidebar.tsx
    app/RightDrawer.tsx
    primitives/...
    chat/...
    agents/...
    research/...
  lib/
    api.ts      # fetch helpers
    sse.ts      # POST-SSE reader
    query.ts    # React Query client
    store.ts    # Zustand slices (ui/theme)
    types.ts    # TS types mirrored from API
  styles/globals.css
  tailwind.config.ts
  .env.local    # NEXT_PUBLIC_API_BASE=http://127.0.0.1:6061
```

## 3) Design System (dark grey / teal)

Color tokens (CSS variables): neutrals for background/surfaces/text; teal accent for interactions; semantic ok/warn/err. Radii, elevation, motion, and typography defined to ensure WCAG‑conformant contrast and predictable focus rings.

Rules:

- Accent usage: primary buttons, links, active states, focus rings.
- Borders: 1px hairlines; favor spacing/elevation over dividers.
- Focus: 2px ring in accent, offset 2px.

## 4) App Shell

- TopNav: product title; tab switcher (Chat/Agents/Research); status pill; Command Palette (⌘K/Ctrl‑K).
- Left Sidebar: contextual per tab (Chat: sessions; Agents: list; Research: filters).
- Main Content: active workspace.
- Right Drawer: Approvals, Memory search, System map digest.

Breakpoints: desktop shows sidebar + content + (optional) drawer; smaller viewports collapse to overlays/sheets.

## 5) Primitives

Button, Input/Textarea, Select/Combobox, Switch, Tabs, Tooltip, Toast, Dialog/Drawer/Popover, Badge/Pill, Card, Skeleton, Table, CodeBlock, EmptyState. Minimal prop surfaces with sensible defaults.

## 6) Pages

Chat:

- SessionList + New Chat; MessageList with streaming tokens; ToolBlock cards inline and in the Activity drawer; Composer with Enter/Shift+Enter, Stop/Retry, Agent picker; inline Approvals banner.
- Acceptance: smooth token streaming; tool call events visible; approvals unblock flow.

Agents:

- Sidebar list; split main with Overview, Config, Tools, Policy, Test.
- For V1, read/act via existing `/api/agents` endpoints (create/list/get/pause/resume/abort/replan/artifacts), with a design path to add update endpoints if needed.

Research:

- Table view with virtualized rows; Task Drawer with Overview/Artifacts/Atoms/Events; brief triggers for `/api/schedules/run/{arxiv,news}`.

## 7) Settings (minimal)

Accent color, UI density, font size, code ligatures, inline tool logs. Persisted to `localStorage` and applied via CSS variables/class toggles.

## 8) Accessibility & UX Quality

Contrast ≥ 4.5:1 for primary text; visible focus; ARIA labels on critical controls; SR live region for streaming; reduced‑motion variants.

## 9) Data & State Patterns

- React Query keys: health/sessions/agents/tasks/tools/approvals.
- SSE Bus: events `token`, `tool_call(s)`, `tool_result`, `error`, `done`; Stop via `AbortController`.
- Error handling: global boundary + toasts; retry affordances.

## 10) Performance

Virtualized lists; debounced inputs; on‑demand Monaco; keep hydration light.

## 11) Security & Localhost

- CORS must allow the web app origin for `application/json` and `text/event-stream`.
- No secrets in frontend.
- Artifact downloads via local authenticated endpoints when needed.

## 12) Acceptance Criteria (end‑to‑end)

Chat:

- Start a new session, send a message, receive streaming reply.
- Tool call appears in Activity drawer; inline block togglable.
- Approval banner can approve/deny and unblocks flow.

Agents:

- Create agent and view details; resume/pause/abort actions.
- “Use in Chat” opens Chat preselected with the agent.

Research:

- Create task; open Task Drawer with artifacts/atoms/events; trigger briefs and see progress.

Settings:

- Switch accent/density/font size/ligatures; app updates instantly; reset to defaults works.

Accessibility:

- Full keyboard support; visible focus; SR announces streaming completion.

## 13) Implementation Roadmap (high‑level)

1. Shell + Theme + Health badge
2. Chat MVP (sessions, streaming, composer, tool logs)
3. Approvals (inline)
4. Agents UI (read + actions; update endpoint if required)
5. Research (table + drawer + brief triggers)
6. Settings (minimal preferences)
7. Final polish (empty states, shortcuts, Monaco fences, toasts)

Refer to plans/PR‑0200..0207 for per‑PR acceptance criteria, tests, and wiring checklists.

