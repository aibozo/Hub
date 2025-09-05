"use client";
import { useEffect, useMemo, useState } from 'react';
import { api } from '@/lib/api';
import type { Task, ArtifactRow } from '@/lib/types';
import { useQuery } from '@tanstack/react-query';
import { useUi } from '@/lib/store';

export function TaskDrawer({ task, onClose }: { task: Task; onClose: () => void }) {
  const [tab, setTab] = useState<'overview' | 'context' | 'artifacts' | 'atoms'>('overview');
  const { data: artifacts } = useQuery({ queryKey: ['task', task.id, 'artifacts'], queryFn: () => api.taskArtifacts(task.id) });
  const { data: atoms } = useQuery({ queryKey: ['task', task.id, 'atoms'], queryFn: () => api.taskAtoms(task.id) });
  const { data: pack } = useQuery({ queryKey: ['task', task.id, 'context-pack'], queryFn: () => api.contextPack(task.id) });
  const pushActivity = useUi((s) => s.pushActivity);

  return (
    <div className="fixed inset-0 z-30 bg-black/40">
      <div className="absolute right-0 top-0 h-full w-[540px] border-l border-border bg-bg shadow-pop">
        <div className="flex items-center justify-between border-b border-border p-3">
          <div>
            <div className="text-sm font-semibold text-text">{task.title}</div>
            <div className="text-xs text-text-dim">{task.status} · {task.tags ?? ''}</div>
          </div>
          <button className="rounded-md bg-surface px-3 py-1 text-sm hover:bg-bg-1" onClick={onClose}>Close</button>
        </div>
        <div className="border-b border-border px-3">
          {(['overview', 'context', 'artifacts', 'atoms'] as const).map((t) => (
            <button key={t} className={`mr-3 pb-2 text-sm ${tab === t ? 'text-text' : 'text-text-dim hover:text-text'}`} onClick={() => setTab(t)}>
              {t.charAt(0).toUpperCase() + t.slice(1)}
              {tab === t && <span className="block h-0.5 w-full bg-accent-500" />}
            </button>
          ))}
        </div>
        <div className="h-[calc(100%-100px)] overflow-y-auto p-3">
          {tab === 'overview' && <Overview task={task} onRun={(job) => api.runBrief(job).then((r) => pushActivity({ ts: Date.now(), kind: 'info', payload: { brief: job, ok: true } })).catch((e) => pushActivity({ ts: Date.now(), kind: 'error', payload: { brief: job, error: String(e) } }))} />}
          {tab === 'context' && <Context pack={pack as any} />}
          {tab === 'artifacts' && <Artifacts items={(artifacts ?? []) as ArtifactRow[]} />}
          {tab === 'atoms' && <Atoms items={(atoms ?? []) as any[]} />}
        </div>
      </div>
    </div>
  );
}

function Overview({ task, onRun }: { task: Task; onRun: (job: 'arxiv'|'news') => void }) {
  return (
    <div className="space-y-3">
      <div className="rounded-md bg-bg-1 p-3 shadow-card">
        <div className="mb-1 text-sm font-medium text-text">Overview</div>
        <div className="text-sm text-text-dim">Created: {new Date(task.created_at).toLocaleString()}</div>
        <div className="text-sm text-text-dim">Updated: {new Date(task.updated_at).toLocaleString()}</div>
      </div>
      <div className="rounded-md bg-bg-1 p-3 shadow-card">
        <div className="mb-2 text-sm font-medium text-text">Run Briefs</div>
        <div className="flex items-center gap-2">
          <button className="rounded-md bg-accent-500 px-3 py-1 text-sm text-black hover:bg-accent-600" onClick={() => onRun('arxiv')}>Run arXiv brief</button>
          <button className="rounded-md bg-accent-500 px-3 py-1 text-sm text-black hover:bg-accent-600" onClick={() => onRun('news')}>Run News brief</button>
        </div>
      </div>
    </div>
  );
}

function Context({ pack }: { pack: any }) {
  if (!pack) return <div className="text-text-dim">No context cards.</div>;
  const cards: any[] = pack.cards ?? [];
  return (
    <ul className="space-y-2">
      {cards.map((c, i) => (
        <li key={i} className="rounded-md bg-bg-1 p-3 shadow-card">
          <div className="mb-1 text-xs text-text-dim">atom #{c.atom_id} · tokens~{c.tokens_est} · {c.pinned ? 'pinned' : `imp ${c.importance}`}</div>
          <pre className="whitespace-pre-wrap break-words text-xs">{c.text}</pre>
        </li>
      ))}
    </ul>
  );
}

function Artifacts({ items }: { items: ArtifactRow[] }) {
  if (!items.length) return <div className="text-text-dim">No artifacts.</div>;
  return (
    <ul className="space-y-2">
      {items.map((a) => (
        <li key={a.id} className="rounded-md bg-bg-1 p-3 shadow-card">
          <div className="text-sm text-text">{a.path}</div>
          <div className="text-xs text-text-dim">{a.mime ?? 'unknown'} · {a.bytes ?? 0} bytes</div>
        </li>
      ))}
    </ul>
  );
}

function Atoms({ items }: { items: any[] }) {
  if (!items.length) return <div className="text-text-dim">No atoms.</div>;
  return (
    <ul className="space-y-2">
      {items.map((a) => (
        <li key={a.id} className="rounded-md bg-bg-1 p-3 shadow-card">
          <div className="mb-1 text-xs text-text-dim">{a.kind} · {new Date(a.created_at).toLocaleString()}</div>
          <pre className="whitespace-pre-wrap break-words text-xs">{a.text}</pre>
        </li>
      ))}
    </ul>
  );
}

