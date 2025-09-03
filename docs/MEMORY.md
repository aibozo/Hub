# Memory Plane

The memory plane preserves recall without destructive compaction by using layered stores and a context packer that composes per-turn context within a fixed token budget.

## Principles

- Never delete facts; prefer summarization and sharding.
- Keep a fixed prompt budget; use expansion handles to progressively stream larger shards on demand.

## Stores

- Event Log (SQLite): append-only records of steps, tool calls, outputs (hashed), approvals.
- Atoms (facts/insights): `{type, text, vector, bm25_terms, tags, task_id, created_at}`.
- Artifacts: files (reports, cached PDFs, screenshots) with `artifact://` URIs and checksums.
  - Research artifacts:
    - `research_report` → Markdown brief at `storage/briefs/<YYYY-MM-DD>-arxiv.md` (+ JSON sidecar)
    - `research_bundle` → packed JSON bundle used for synthesis and traceability
- Indices: Tantivy BM25 + HNSW/FAISS embeddings, namespaced by `/global`, `/task/<id>`, `/spec/*`.

## Task Working Memory

- Each Task has a one-paragraph digest refreshed on change and a short form (1–2 sentences).
- Global knowledge cards: recurring facts (preferences, env quirks, installed tools) in 1–3 sentences.

## Context Packer

- Inputs: system map digest + active task digest + top‑K global cards + requested expansions (bounded).
- Budgeting: drop least-critical expansions first, then trim cards, until under token budget.
- Expansion mechanic: `expand://task/<id>?depth=n` progressively streams larger shards, with budget checks each step.

Pseudo-code:

```
pack(context_budget):
  sys = digest(system_map)
  task = active_task.paragraph
  cards = topK(global_cards, k=6)
  expansions = requested_expansions()
  body = [sys, task, cards, expansions]
  while tokens(body) > context_budget:
    drop_tail(body)
  return join(body)
```

## Schema (SQLite)

```
Task(id, title, status, created_at, updated_at, tags)
TaskDigest(task_id, short, paragraph, tokens)
Atom(id, task_id, kind, text, vector, bm25, tags, created_at)
Artifact(id, task_id, path, mime, sha256, created_at)
Event(id, task_id, kind, payload_json, created_at)
```

## Indexing

- BM25 via Tantivy with token filters tuned for code+English.
- Embeddings via local model; vectors persisted for HNSW search.
- Periodic re-index jobs if thresholds crossed; incremental updates for atom/appends.

## Privacy

- Redact secrets by policy; never embed raw secrets; keep artifacts out of prompts unless user approves.
