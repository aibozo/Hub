# Personal Assistant “Foreman” — System Design v0.1

A modular, always-on local assistant that you can drive from a TUI or voice, orchestrating tools via MCP with strong safety gates, rich memory, and research/reporting agents. Designed to be extended as your ideas grow.

---

## 0) Goals & Non‑Goals

**Goals**

* Always-on assistant with wake word + live voice, controllable from a Rust TUI.
* Runs locally; can execute commands, open apps, manage processes, install software (with approvals), launch emulators/games.
* Modular tools via MCP servers; easy to add new capabilities.
* Strong safety rails (no dangerous deletes/`sudo` without explicit approval + provenance/explainers).
* High-level “system map” of your machine pinned to context, plus per-tool interaction instructions.
* Hierarchical memory: task working memory, long-term store, and context packer that stays within token budgets while preserving full recall (no destructive compacts).
* Research agents: arXiv and news digesters with daily briefs + cached summaries for deep follow-ups.
* Debate mode (“Deep Argue”): two-stance agents with source-grounded arguments and a reasoned report.
* Spec Help Mode: structured project/spec planning that reads your repos and outputs docs/architecture plans.

**Non‑Goals (for v1)**

* Full GUI or mobile app (use a PWA bridge + push as interim).
* Unfettered automated desktop control (keep explicit gates/approvals).
* Cloud-only dependencies; prefer local/edge components with optional cloud LLMs.

---

## 1) Architecture Overview

**Processes**

1. **assistant-core (Rust)** — Orchestrator. Hosts policy engine, task manager, memory packer, MCP client, web server for TUI/mobile bridge.
2. **voice-daemon (Python)** — Wake word (openWakeWord), VAD, STT (whisper.cpp or faster local), TTS (piper). Talks to core via gRPC/WebSocket.
3. **mcp-servers/** — Tool servers (shell/fs, git/worktree, arxiv, news, spotify, websearch, emulator+steam, installer, spec-agent, debate-agent, etc.). Each is replaceable.
4. **ui-tui (Rust/ratatui)** — Terminal client. Hotkeys, chat, task lists, memory search, approvals, /voice.
5. **storage** — SQLite (event log, tasks, configs), Tantivy (sparse full-text), HNSW/FAISS (embeddings), object store (reports, cached PDFs, artifacts).
6. **mobile-bridge (PWA)** — Simple Next.js static app connecting back via WebSocket + push (ntfy/self-host or APNs proxy later).

**Data Flows**

* Voice → STT → assistant-core → plan → gated tool calls (via MCP) → results → memory update → TTS response.
* TUI commands → assistant-core → same flow.
* Schedulers (cron-like in core) trigger daily arXiv/news jobs → summaries cached → morning brief.

---

## 2) Safety & Policy (“Gatekeeper”)

**Command classes**

* **Safe:** read-only, list, open, fetch, search.
* **Warn:** write within user home, package installs from trusted repos.
* **Block-by-default:** `sudo`, system-level changes, deletes outside whitelisted paths, network changes.

**Approval flow**

1. Tool proposes an action with a **plan card** ({why, exact command, paths, sources}).
2. Core evaluates against **policy rules** (YAML). If risky → **HOLD**.
3. User approves/denies in TUI (or voice via confirmation). Optionally **Explain This** → provenance explainer shows source, hash, vendor, package origin, and safety notes.
4. On approval: core issues command with **supervision** (pty wrapper, timeout, resource limits, logging).

**Dry-run support**

* Every executor implements `--dry-run` where possible; else simulate (e.g., `apt-get -s`, `pip install --dry-run`).

**Quarantine**

* Downloads saved to `~/Foreman/Quarantine/<sha256>`; scanned (file type, checksum) before moving to target.

---

## 3) System Map (Pinned Context)

A static digest plus detail-on-demand describing the machine and how to interact with it.

**Inventory scanner** (runs daily/on change) outputs `map.json`:

* **Hardware:** CPU/GPU, RAM, disks, audio I/O devices.
* **OS:** distro, kernel, service manager, shells, locales, timezone.
* **Runtimes:** Python, Rust, Node, CUDA, Java versions.
* **Package managers:** apt, snap, flatpak, pip, cargo, npm, conda.
* **Apps/CLIs:** emulators (mGBA, melonDS), Steam, browsers, editors (nvim, code), docker/podman.
* **Dev envs:** git repos, worktrees, language servers.
* **Network:** interfaces, local domains, open ports (limited for privacy), local services (Home Assistant, etc.).

**Pinned Context (compact)**

* A \~200–400 token “System Map Digest” that names components and the **approved interaction rules** for each (e.g., “Use `steam -applaunch` for games; emulator CLI wrappers below; apt installs require approval.”).
* Links/URIs to detail pages: `map://packages`, `map://emulators`, `map://worktrees` (resolved locally by core, not sent to LLM unless expanded).

---

## 4) Memory Plane

**Principles**

* Never delete facts; we *summarize*, *shard*, and *index* for recall.
* Keep a fixed prompt budget by composing a **Context Pack** per turn.

**Stores**

* **Event Log (append-only, SQLite):** all steps, tool calls, outputs (hashed), approvals.
* **Atoms (facts/insights):** typed memory records with `{type, text, embeddings, sparse terms, tags, task_id, created_at}`.
* **Artifacts:** files, reports, PDFs, cached HTML, screenshots.
* **Indices:** Tantivy (BM25) + HNSW (embeddings) per namespace (`/global`, `/task/<id>`, `/spec/*`).

**Working memory**

* Each **Task** has a **one-paragraph digest** refreshed on change.
* Global **knowledge cards**: 1–3 sentence summaries of recurring facts (preferences, env quirks, installed tools).

**Context packer (budgeter)**

* Inputs: `system_digest` + `active_task_digest` + top‑K global cards + per-turn “expansions”.
* **Expansion mechanic:** the agent may request `Expand(task=<id>, depth=n<=10)`; core streams in progressively larger shards (paragraph → page → full transcript), each time checking token budget.
* If many tasks exist, the **search-overview** inserts *one sentence* per task, with an inline `expand://task/<id>` handle the agent can call.

---

## 5) Task Lifecycle

1. **Start** (implicit): any user request spawns/attaches to a Task with `intent`, `inputs`, `constraints`.
2. **Plan**: tool-free outline unless simple.
3. **Execute**: gated tool calls; checkpoint after each meaningful step.
4. **Report**: concise outcome + links to artifacts.
5. **Commit**: update task digest + emit 1-sentence global summary if broadly useful.

Schema (SQLite):

```
Task(id, title, status, created_at, updated_at, tags)
TaskDigest(task_id, short, paragraph, tokens)
Atom(id, task_id, kind, text, vector, bm25, tags, created_at)
Artifact(id, task_id, path, mime, sha256, created_at)
Event(id, task_id, kind, payload_json, created_at)
```

---

## 6) Tools via MCP (v1 set)

**Core**

* `mcp-shell`: read-only and write modes; supports dry-run, cwd selection, env whitelist.
* `mcp-fs`: list/read/write with path policy.
* `mcp-proc`: list/kill/renice; safe defaults.
* `mcp-git`: status, worktrees, branch create/switch; diff summaries.

**Research**

* `mcp-arxiv`: search by query, date ranges; fetch PDFs; summarize to cache; “top N of month” by citation proxy (crossref or internal citation count cache); daily brief job.
* `mcp-news`: curated feeds + dedup + category tags (geopolitical, markets, tech policy); daily brief; crisis alerts.

**Media**

* `mcp-spotify`: auth, now playing, queue, playlists.

**Desktop/Games**

* `mcp-steam`: `steam -applaunch <id>`, library list.
* `mcp-emu`: wrappers for mGBA, melonDS, PCSX2; per-game profiles; save-state ops.

**System mgmt**

* `mcp-installer`: apt/snap/flatpak/pip/cargo with `plan` + `explain` + `dry-run` + `approve`.
* `mcp-open`: open files/URLs/apps cross-desktop (`xdg-open` abstraction).

**Spec & Debate**

* `mcp-spec`: repo scan → integration plan/spec doc generator; emits architecture + plan files.
* `mcp-debate`: “Deep Argue” orchestrator (two stances + sources) → report with logs.

**Web**

* `mcp-websearch`: pluggable engines; returns URL + snippet + metadata; fetcher with robots-aware caching.

---

## 7) Voice & Wake Word

* **Wake word:** openWakeWord model (local), customizable phrase.
* **VAD:** WebRTC VAD.
* **STT:** whisper.cpp (medium/small) with dynamic length limits for low latency; fallback to remote STT if opted in.
* **TTS (stack):** Primary CosyVoice 2 (streaming), OpenVoice V2 for cloning/style control, Kokoro (ONNX) + Piper as CPU fallbacks.
* **Hotkeys:** in TUI, `/voice` toggles; while TUI open, wake word also triggers.

---

## 7.1) TTS Integration Plan (CosyVoice2/OpenVoice/Kokoro/Piper)

**Goals**: low first-audio latency, expressive prosody, license-aware engine selection, seamless fallback, and barge‑in (interrupt playback on user speech/wake).

### Directory layout (Python voice-daemon)

```text
mcp-servers/python/voice_daemon/
├─ pyproject.toml
├─ voice_daemon/
│  ├─ __main__.py            # starts WS server, loads config, registers engines
│  ├─ server.py              # WebSocket + simple HTTP endpoints
│  ├─ schemas.py             # pydantic models for requests/responses
│  ├─ router.py              # engine selection + streaming orchestration
│  ├─ audio.py               # resampling, chunking, wav mux
│  ├─ cache.py               # voice refs, small phrase cache (hash→pcm)
│  ├─ ssml_lite.py           # [pause:], [rate:], [pitch:] etc → engine controls
│  └─ engines/
│     ├─ base.py             # Engine abstract base class
│     ├─ cosyvoice2.py       # streaming client
│     ├─ openvoice.py        # cloning + style control
│     ├─ kokoro.py           # ONNX Runtime synth
│     └─ piper.py            # wraps piper binary (subprocess)
└─ models/                   # downloaded models (gitignored)
```

### Engine interface

```python
# voice_daemon/engines/base.py
from typing import AsyncIterator, Optional, Dict

class TtsCapabilities:
    streaming: bool
    clone: bool
    markers: list[str]  # supported SSML-lite tags
    license: str        # e.g., "MIT", "Apache-2.0", "NC"

class TtsEngine:
    name: str
    async def preload(self) -> None: ...
    def capabilities(self) -> TtsCapabilities: ...
    async def synth_stream(self, text: str, *, voice_id: str | None = None,
                            ref_path: str | None = None, lang: str | None = None,
                            prosody: Dict | None = None) -> AsyncIterator[bytes]:
        """Yield 16‑bit PCM chunks at 24k/mono (engine-resampled if needed)."""
    async def synth_file(self, text: str, **kw) -> str: ...
```

### WS protocol (assistant-core ↔ voice-daemon)

* **WS path**: `/v1/tts/stream`
* **Client → server (JSON)**:

  ```json
  {"request_id":"uuid","text":"Hello [pause:200ms] world.","voice":"cosy/en-neutral",
   "ref":"~/Voices/riley_ref.wav","engine":"cosyvoice2",
   "allow_nc":false,"sample_rate":24000}
  ```
* **Server → client frames**:

  * `{"type":"chunk","seq":n,"pcm_b64":"..."}`
  * `{"type":"metrics","t_first_ms":145,"rtf":0.35}`
  * `{"type":"done"}` or `{"type":"error","message":"..."}`

### Rust audio output service

* **Crate**: part of `apps/assistant-core` as `audio_out.rs` using `cpal` + `rodio`.
* **Flow**: WS client task connects → decodes base64 PCM → pushes to ring buffer → rodio `Decoder` from stream → device playback.
* **Barge‑in**: a shared atomic flag cancels current stream on wake word or `/stop`.

```rust
// assistant-core/src/audio_out.rs (sketch)
pub struct AudioOut { /* device, sink, tx */ }
impl AudioOut {
    pub fn play_stream(&self, rx: crossbeam_channel::Receiver<Vec<i16>>) { /* feed sink */ }
    pub fn stop(&self) { /* stop sink / drain */ }
}
```

### SSML‑lite markup

* Tags: `[pause:200ms]`, `[rate:+10%]`, `[pitch:-2st]`, `[style:calm]`, `[emph]...[/emph]`.
* Mapper in `ssml_lite.py` converts tags into engine‑native controls (e.g., token inserts for Chat‑style models or param knobs for OpenVoice/CosyVoice).

### Config (extend `config/foreman.toml`)

```toml
[tts]
primary = "cosyvoice2"
fallbacks = ["kokoro", "piper"]
license_allow_nc = false
sample_rate = 24000
chunk_ms = 80            # target chunk duration

[tts.cosyvoice2]
endpoint = "http://127.0.0.1:6060"
voice = "en-neutral"

[tts.openvoice]
enabled = true
ref = "~/Voices/riley_ref.wav"
style = { emotion = "calm", accent = "us", rhythm = "neutral" }

[tts.kokoro]
onnx_model = "~/models/kokoro.onnx"

[tts.piper]
binary = "/usr/local/bin/piper"
voice_model = "~/voices/en_US-amy-medium.onnx"
```

### Bootstrap script

`scripts/bootstrap.sh` will:

* Create Python venv for voice-daemon (`uv venv`), install deps.
* Pull/convert Kokoro ONNX and Piper voice.
* Print a one‑liner to start CosyVoice/OpenVoice servers (if external) or fetch docker images.

### Benchmarks & Health

* `GET /v1/tts/health` → engine readiness + preload time.
* `POST /v1/tts/bench` → read the Rainbow Passage; returns `t_first_ms`, `rtf`, CPU/GPU mem.
* Core logs metrics (`tracing`) and exposes Prometheus via `telemetry.rs`.

### Tests

* Python: pytest async tests per engine (synth small text → non‑empty PCM; resample to 24k; RMS sane).
* Rust: integration test connects to WS, plays 1s tone, asserts no panic & timely first chunk.

### Fallback & selection logic

1. If `engine` specified but not ready → try next in `fallbacks`.
2. If GPU busy/no CUDA → prefer Kokoro→Piper.
3. If `license_allow_nc=false`, engines marked NC are skipped.
4. For very short prompts (<200ms speech), enable **phrase cache** (hash→pcm).

### TUI controls

* `/voice test` — say a demo line with current engine.
* `/voice engine list|set cosyvoice2` — switch engines.
* `/voice style` — list presets (from OpenVoice/CosyVoice).
* `space` — barge‑in stop; `r` — repeat last ut

## 8) TUI (ratatui)

**Screens**

* **Chat:** conversation stream with inline tool cards and approval prompts.
* **Tasks:** active/past with statuses; press `Enter` to expand; `a` approve, `x` deny, `?` explain.
* **Memory:** search, open cards, pin/unpin; visualize context pack.
* **Tools:** quick launchers (emulators, steam games, installers).
* **Settings:** wake word, STT/TTS, policy toggles, schedules.

**Keymap**

* `Ctrl-k` focus command line, `/voice` toggle, `/task new`, `/install <pkg>`, `/open <app>`, `/emu <rom>`, `/debate <topic>`, `/spec <dir>`.

---

## 9) Schedulers & Daily Briefs

* Cron-like in `assistant-core` with timezone **America/Indiana/Indianapolis**.
* **07:30** arXiv sweep (AI/ML, systems, robotics + “wildcard of the day”).
* **08:00** News risk brief (geo/US policy/markets), with 5–10 bullets and 2–3 “watch” items.
* Cache full summaries; linkable via `artifact://` URIs in memory.

---

## 10) Deep Argue (two-agent debate)

**Inputs**: topic prompt, stance A/B seeds, search budget, max rounds.

**Flow**

1. Gather sources (balanced; both agents share a citation pool + can add).
2. Alternating arguments with a judge summarizer. Agents must track concessions.
3. Termination when one cedes or max rounds hit.
4. Output: structured report with claims, evidence, concessions, unresolved points, and practical implications.

**Safeguards**: source-grounding required for factual claims; judge penalizes uncited assertions.

---

## 11) Spec Help Mode

**Mode**: switch context to *Project Planner*.

**Capabilities**

* Read a repo (path whitelist), detect language/services, extract boundaries.
* Ask targeted questions to fill unknowns.
* Produce: (1) plan.md, (2) architecture.md, (3) integration.md diffs if needed.
* Optionally open PR or patch files (gated by approval).

---

## 12) Data Schemas & Config

**Config (TOML)**

```
[foreman]
home = "~/Foreman"
profile = "default"

[voice]
wake_phrase = "hey foreman"
stt = { engine = "whisper.cpp", model = "medium" }
tts = { engine = "piper", voice = "en_US-libritts-high" }

[policy]
protect_paths = ["/", "/etc", "/usr", "/var", "/boot"]
write_whitelist = ["~/", "/mnt/data/projects"]
require_approval = ["sudo", "apt", "snap", "flatpak", "pip", "cargo", "rm -rf", "pkill", "iptables"]

[schedules]
arxiv_brief = "07:30"
news_brief  = "08:00"

[mcp]
servers = ["shell", "fs", "proc", "git", "arxiv", "news", "spotify", "steam", "emu", "installer", "websearch", "spec", "debate"]
```

**MCP Tool Manifest (example)**

```json
{
  "name": "mcp-installer",
  "tools": [
    {"name": "plan_install", "input": {"pkg": "string", "manager": "string?"}},
    {"name": "explain_install", "input": {"plan_id": "string"}},
    {"name": "apply_install", "input": {"plan_id": "string", "approve_token": "string"}},
    {"name": "dry_run", "input": {"plan_id": "string"}}
  ]
}
```

---

## 13) Context Packer Pseudocode

```
pack(context_budget):
  sys = digest(system_map)
  task = active_task.paragraph
  cards = topK(global_cards, k=6)
  expansions = requested_expansions()  # bounded size
  body = [sys, task, cards, expansions]
  while tokens(body) > context_budget:
      drop_tail(body)  # remove least-critical expansions, then trim cards
  return join(body)
```

**Expansion**

```
expand(handle, depth):
  for i in 1..depth:
    chunk = next_chunk(handle)
    if would_exceed_budget(chunk): break
    inject(chunk)
```

---

## 14) Repo Layout (v0.2 — Detailed File/Directory Plan)

Two layers: a Rust workspace for the **core + TUI + Rust tools**, and a Python workspace for **voice + research MCP servers**. Safe defaults, batteries included, and room to grow.

```text
foreman/
├─ Cargo.toml                      # Workspace root (Rust)
├─ rust-toolchain.toml             # Pin toolchain
├─ Justfile                        # dev tasks (build/run/test/format)
├─ Makefile                        # optional; mirrors Justfile targets
├─ .editorconfig
├─ .gitignore
├─ .env.example
├─ README.md
├─ LICENSE
│
├─ apps/
│  ├─ assistant-core/              # Rust orchestrator daemon (bin)
│  │  ├─ Cargo.toml
│  │  ├─ src/
│  │  │  ├─ main.rs                # boots services; loads config; starts schedulers
│  │  │  ├─ app.rs                 # wiring of subsystems
│  │  │  ├─ api.rs                 # WS/HTTP control (for PWA/TUI bridge)
│  │  │  ├─ mcp_client.rs          # stdio/WebSocket MCP client
│  │  │  ├─ planner.rs             # high-level planning loop + tool gating
│  │  │  ├─ gatekeeper/            # safety policy layer
│  │  │  │  ├─ mod.rs
│  │  │  │  ├─ policy.rs           # YAML policy loading/merging
│  │  │  │  ├─ approvals.rs        # queues, tokens, expirations
│  │  │  │  └─ provenance.rs       # Explain This (sources, hashes, dry-run output)
│  │  │  ├─ exec/                  # supervised subprocess runner
│  │  │  │  ├─ mod.rs
│  │  │  │  ├─ sandbox.rs          # cwd/path guards, env allowlist, timeouts
│  │  │  │  └─ pty.rs              # interactive sessions
│  │  │  ├─ memory/
│  │  │  │  ├─ mod.rs
│  │  │  │  ├─ atoms.rs            # fact/insight records
│  │  │  │  ├─ events.rs           # append-only event log
│  │  │  │  ├─ indices.rs          # tantivy (BM25) + HNSW vectors
│  │  │  │  ├─ context_pack.rs     # budgeter + expansion handles
│  │  │  │  └─ schema.rs           # SQLite schema & migrations
│  │  │  ├─ system_map/
│  │  │  │  ├─ mod.rs
│  │  │  │  ├─ scan.rs             # hardware/OS/runtimes/apps enumerator
│  │  │  │  ├─ digest.rs           # ~200–400 token pinned digest
│  │  │  │  └─ model.rs            # types (devices, packages, tools)
│  │  │  ├─ scheduler.rs           # cron-like jobs (arXiv/news)
│  │  │  ├─ artifacts.rs           # file store + URIs (artifact://)
│  │  │  ├─ config.rs              # foreman.toml parsing
│  │  │  └─ telemetry.rs           # tracing/logging/metrics
│  │  ├─ migrations/               # SQL (sqlx) migrations for SQLite
│  │  └─ tests/                    # integration tests
│  │
│  └─ ui-tui/                      # Rust TUI (bin)
│     ├─ Cargo.toml
│     ├─ src/
│     │  ├─ main.rs
│     │  ├─ app.rs                 # screens router
│     │  ├─ components/            # panels, tables, statusline, dialogs
│     │  ├─ screens/
│     │  │  ├─ chat.rs
│     │  │  ├─ tasks.rs
│     │  │  ├─ memory.rs
│     │  │  ├─ tools.rs
│     │  │  └─ settings.rs
│     │  ├─ theme.rs               # theme/skins
│     │  └─ keymap.rs
│     └─ assets/
│        └─ templates/             # brief/report markdown templates
│
├─ crates/                         # Shared Rust libs (clean seams)
│  ├─ foreman-types/               # common types (Tasks, Atoms, Policy, SystemMap)
│  │  ├─ Cargo.toml
│  │  └─ src/lib.rs
│  ├─ foreman-config/              # TOML config + validation
│  ├─ foreman-policy/              # rule evaluation (safe/warn/block)
│  ├─ foreman-memory/              # memory adapters (SQLite/Tantivy/HNSW)
│  ├─ foreman-mcp/                 # MCP protocol client (serde types + transports)
│  ├─ foreman-exec/                # supervised exec primitives (shared with servers)
│  ├─ foreman-system-map/          # scanning/digest utilities
│  └─ foreman-telemetry/           # tracing + metrics setup
│
├─ mcp-servers/
│  ├─ rust/
│  │  ├─ shell/
│  │  │  ├─ Cargo.toml
│  │  │  └─ src/main.rs            # provides ls/cat/exec (with guard hooks)
│  │  ├─ fs/
│  │  ├─ proc/
│  │  ├─ git/
│  │  ├─ emu/
│  │  └─ steam/
│  │
│  └─ python/
│     ├─ voice_daemon/
│     │  ├─ pyproject.toml
│     │  ├─ voice_daemon/
│     │  │  ├─ __init__.py
│     │  │  ├─ __main__.py         # WS server: wake/VAD/STT/TTS → core
│     │  │  ├─ wake.py             # openWakeWord
│     │  │  ├─ stt.py              # faster-whisper
│     │  │  ├─ vad.py              # webrtcvad
│     │  │  └─ tts.py              # piper
│     │  └─ models/                # model files (gitignored; managed by setup)
│     ├─ arxiv_server/
│     │  ├─ pyproject.toml
│     │  └─ arxiv_server/
│     │     ├─ __main__.py         # MCP stdio server
│     │     ├─ search.py
│     │     ├─ ingest.py           # PDF fetch/cache
│     │     └─ summarize.py        # cache summaries → artifacts
│     ├─ news_server/
│     ├─ websearch_server/
│     ├─ installer_server/
│     ├─ spec_server/
│     └─ debate_server/
│
├─ config/
│  ├─ foreman.toml                 # main config
│  ├─ policy.d/
│  │  ├─ 00-defaults.yaml          # protect_paths, approvals, limits
│  │  └─ 10-local-overrides.yaml
│  ├─ tools.d/                     # MCP server manifests and endpoints
│  │  ├─ shell.json
│  │  ├─ fs.json
│  │  ├─ proc.json
│  │  ├─ git.json
│  │  ├─ arxiv.json
│  │  ├─ news.json
│  │  ├─ websearch.json
│  │  ├─ installer.json
│  │  ├─ emu.json
│  │  └─ steam.json
│  ├─ schedules.toml               # cron-like job times
│  └─ tui.toml                     # theme, keymap
│
├─ storage/
│  ├─ sqlite.db                    # event log + atoms
│  ├─ indices/                     # tantivy + vector index files
│  ├─ artifacts/                   # reports, cached PDFs, screenshots
│  ├─ quarantine/                  # downloaded files awaiting approval
│  └─ logs/
│
├─ docs/
│  ├─ ARCHITECTURE.md
│  ├─ POLICY.md
│  ├─ MEMORY.md
│  ├─ SYSTEM_MAP.md
│  ├─ TOOLS.md
│  └─ ADR/                         # Architecture Decision Records
│
├─ scripts/
│  ├─ bootstrap.sh                 # create venvs, fetch models, pre-commit hooks
│  ├─ dev-run.sh                   # tmux/foreman to run core + TUI + servers
│  ├─ install-rust.sh
│  ├─ install-python.sh
│  ├─ migrate.sh                   # sqlx migrations
│  └─ pack-assets.sh
│
├─ .github/
│  └─ workflows/
│     ├─ ci.yml                    # build/test both Rust and Python
│     └─ lint.yml
│
└─ pwa/                            # (later) Next.js bridge (optional for v1)
   ├─ package.json
   ├─ next.config.mjs
   └─ src/
```

### Workspace `Cargo.toml` (root)

```toml
[workspace]
members = [
  "apps/assistant-core",
  "apps/ui-tui",
  "crates/foreman-types",
  "crates/foreman-config",
  "crates/foreman-policy",
  "crates/foreman-memory",
  "crates/foreman-mcp",
  "crates/foreman-exec",
  "crates/foreman-system-map",
  "crates/foreman-telemetry",
  "mcp-servers/rust/shell",
  "mcp-servers/rust/fs",
  "mcp-servers/rust/proc",
  "mcp-servers/rust/git",
  "mcp-servers/rust/emu",
  "mcp-servers/rust/steam",
]
resolver = "2"
```

### Key file purposes (quick map)

* **assistant-core/src/main.rs** — parses config, builds services (policy/memory/system‑map), connects MCP servers, starts schedulers and WS control endpoint.
* **assistant-core/src/gatekeeper/** — policy engine, approvals, Explain‑This provenance and dry‑run integration.
* **assistant-core/src/memory/** — atoms/events/artifacts APIs + context packer and expansion handles.
* **assistant-core/src/system\_map/** — scanners (lsb\_release, lspci, `which`, `--version`), digest builder.
* **ui-tui/** — chat & approvals UI; Tasks/Memory/Tools/Settings screens; `/voice` toggle.
* **crates/** — shared types/utilities so both core and servers stay consistent (e.g., `TaskDigest`, `PolicyRule`, `ArtifactRef`).
* **mcp-servers/** — separate processes (Rust or Python) with simple manifests under `config/tools.d/`.
* **config/** — declarative knobs (policies, schedules, tool endpoints, themes).
* **storage/** — persistent data; safe to back up.

### Python workspace notes

Each Python server is a self-contained package with `pyproject.toml`. Prefer **uv** for lightweight envs. Shared helpers can live in a small internal package (`python/common/foreman_py/`) if helpful.

### Dev ergonomics

* **Justfile** task examples: `just bootstrap`, `just run`, `just run-core`, `just run-tui`, `just run-servers`, `just test`, `just fmt`, `just lint`.
* **scripts/dev-run.sh** can spin up a tmux session with panes for core, TUI, and key servers (voice, arxiv, news).

### Minimal configs (samples)

* `config/foreman.toml` defines wake word, STT/TTS engines, schedules, MCP endpoints.
* `config/policy.d/00-defaults.yaml` ships with conservative protections (`sudo`, `rm -rf`, system paths).
* `config/tools.d/*.json` provide MCP tool schemas and executable paths.

### Lean start vs full split

If you want to start lean, we can collapse `crates/*` into modules inside `assistant-core` and split later. The directory map above is the **target shape**; the MVP can boot with:

* `apps/assistant-core` (with memory, policy, system\_map modules inline),
* `apps/ui-tui`,
* `mcp-servers/rust/shell`, `mcp-servers/python/voice_daemon`,
* `config/`, `storage/`, `scripts/`.

---

## 15) Build Plan (4 Sprints)

**Sprint 1: Skeleton & Safety**

* assistant-core scaffolding, SQLite event log, TUI shell, MCP client.
* mcp-shell/fs/proc/git basic.
* Policy engine with approvals + Explain This (stubbed provenance).
* System Map scanner + pinned digest.

**Sprint 2: Voice & Memory**

* voice-daemon with wake + STT/TTS.
* Memory plane: Atoms + indices; Context packer; Task digests.
* TUI: Tasks+Memory screens; approval UX.

**Sprint 3: Research & Desktop**

* mcp-arxiv/news with schedulers; artifacts + daily briefs.
* mcp-emu/steam launchers; mcp-installer with dry-run and approvals.

**Sprint 4: Modes**

* Spec Help Mode: repo scan → docs.
* Deep Argue: two-agent orchestration + judge + report.
* Mobile PWA bridge (notifications + simple commands).

---

## 16) Risks & Mitigations

* **Token budgets:** Strict context packer + expand-on-demand.
* **Safety:** Default-deny on destructive ops; human-in-the-loop approvals; dry-run.
* **Latency (voice):** Use small STT models; preemptive VAD; incremental decoding.
* **Drift in system map:** Daily scan + on-demand refresh.
* **News/ArXiv spam:** Source curation + dedup + rate limits.

---

## 17) Quick Start (Dev)

1. `cargo new assistant-core && cargo new ui-tui`
2. `python -m venv venv && pip install openwakeword webrtcvad faster-whisper piper-phonemizer`
3. Start `assistant-core` (stub routes), then TUI.
4. Add `mcp-shell` server and run first safe commands (ls, cat).
5. Wire policy approvals; try `/install ripgrep` → dry-run → explain → approve.

---

## 18) Next Steps (Pick One to Implement First)

* **A)** Sprint 1 skeleton (core + TUI + policy + system map).
* **B)** Memory plane + context packer.
* **C)** Research feeders (arXiv/news) + brief templates.



