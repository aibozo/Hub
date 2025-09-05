"use client";
import { useEffect, useMemo, useRef, useState } from 'react';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { api, API_BASE } from '@/lib/api';
import type { AgentSnapshot, ArtifactRow, ToolStatus } from '@/lib/types';
import { readSSE } from '@/lib/sse';
import { useRouter } from 'next/navigation';
import { useUi } from '@/lib/store';

export function AgentDetail({ id }: { id: string }) {
  const qc = useQueryClient();
  const { data, isLoading, isError } = useQuery({ queryKey: ['agent', id], queryFn: () => api.getAgent(id) });
  const [tab, setTab] = useState<'overview' | 'events' | 'artifacts' | 'tools'>('overview');
  const [events, setEvents] = useState<any[]>([]);
  const [artifacts, setArtifacts] = useState<ArtifactRow[]>([]);
  const [tools, setTools] = useState<ToolStatus[]>([]);

  useEffect(() => {
    if (data?.events) setEvents(data.events);
  }, [data?.events]);

  useEffect(() => { api.agentArtifacts(id).then(setArtifacts).catch(() => setArtifacts([])); }, [id]);
  useEffect(() => { api.toolsStatus().then(setTools).catch(() => setTools([])); }, []);

  useEffect(() => {
    let cancel = false;
    const run = async () => {
      try {
        const res = await fetch(`${API_BASE}/api/agents/${id}/events`, { cache: 'no-store' } as any);
        for await (const evt of readSSE(res) as any) {
          if (cancel) break;
          const name = (evt as any).event;
          if (name === 'ping') continue;
          setEvents((prev) => [...prev, { kind: name, ts: new Date().toISOString(), payload: (evt as any).data }]);
        }
      } catch (_) {}
    };
    run();
    return () => { cancel = true; };
  }, [id]);

  if (isLoading) return <div className="rounded-md bg-bg-1 p-6 shadow-card text-text-dim">Loading…</div>;
  if (isError || !data) return <div className="rounded-md bg-bg-1 p-6 shadow-card text-[color:var(--err)]">Failed to load agent.</div>;

  const a = data.agent;
  return (
    <div className="flex h-full flex-col">
      <header className="mb-3 flex items-center justify-between">
        <div>
          <div className="text-lg font-semibold text-text">{a.title}</div>
          <div className="text-xs text-text-dim">{a.model ?? 'default model'} · {a.root_dir}</div>
        </div>
        <div className="flex items-center gap-2">
          <StatusPill status={a.status} />
          <AgentActions id={id} />
          <UseInChat id={id} title={a.title} />
        </div>
      </header>
      <div className="mb-3 border-b border-border">
        {(['overview', 'events', 'artifacts', 'tools'] as const).map((t) => (
          <button key={t} className={`mr-3 pb-2 text-sm ${tab === t ? 'text-text' : 'text-text-dim hover:text-text'}`} onClick={() => setTab(t)}>
            {t.charAt(0).toUpperCase() + t.slice(1)}
            {tab === t && <span className="block h-0.5 w-full bg-accent-500" />}
          </button>
        ))}
      </div>
      <div className="flex-1 overflow-y-auto">
        {tab === 'overview' && <Overview snapshot={data} />}
        {tab === 'events' && <EventsList items={events} />}
        {tab === 'artifacts' && <ArtifactsList items={artifacts} />}
        {tab === 'tools' && <ToolsStatus items={tools} />}
      </div>
    </div>
  );
}

function StatusPill({ status }: { status: string }) {
  return <span className="rounded-full bg-surface px-2 py-0.5 text-2xs text-text-dim">{status}</span>;
}

function AgentActions({ id }: { id: string }) {
  const qc = useQueryClient();
  return (
    <div className="flex items-center gap-2">
      <button className="rounded-md bg-surface px-3 py-1 text-sm hover:bg-bg-1" onClick={async () => { await api.agentPause(id); qc.invalidateQueries({ queryKey: ['agent', id] }); }}>Pause</button>
      <button className="rounded-md bg-[color:var(--ok)] px-3 py-1 text-sm text-black hover:opacity-90" onClick={async () => { await api.agentResume(id); qc.invalidateQueries({ queryKey: ['agent', id] }); }}>Resume</button>
      <button className="rounded-md bg-[color:var(--err)] px-3 py-1 text-sm text-black hover:opacity-90" onClick={async () => { await api.agentAbort(id); qc.invalidateQueries({ queryKey: ['agent', id] }); }}>Abort</button>
      <ReplanQuick id={id} />
    </div>
  );
}

function UseInChat({ id, title }: { id: string; title: string }) {
  const router = useRouter();
  const setSelectedChatId = useUi((s) => s.setSelectedChatId);
  return (
    <button
      className="rounded-md bg-accent-500 px-3 py-1 text-sm text-black hover:bg-accent-600"
      onClick={async () => {
        const s = await api.createSession();
        await api.appendMessage(s.id, { role: 'system', content: `Using agent ${title} (${id})` });
        setSelectedChatId(s.id);
        router.push('/');
      }}
    >
      Use in Chat
    </button>
  );
}

function Overview({ snapshot }: { snapshot: AgentSnapshot }) {
  const a = snapshot.agent;
  return (
    <div className="space-y-3">
      <div className="rounded-md bg-bg-1 p-3 shadow-card">
        <div className="mb-1 text-sm font-medium text-text">Summary</div>
        <div className="text-sm text-text-dim">Model: {a.model ?? 'default'} · Root: {a.root_dir} · Auto-approval: {a.auto_approval_level}</div>
        {a.plan_artifact_id && <div className="text-xs text-text-dim">Plan artifact id: {a.plan_artifact_id}</div>}
      </div>
    </div>
  );
}

function EventsList({ items }: { items: any[] }) {
  if (!items.length) return <div className="rounded-md bg-bg-1 p-3 shadow-card text-text-dim">No events yet.</div>;
  return (
    <ul className="space-y-2">
      {items.map((ev, i) => (
        <li key={i} className="rounded-md bg-bg-1 p-3 shadow-card">
          <div className="mb-1 text-xs text-text-dim">{ev.ts ?? ''} · {ev.kind}</div>
          <pre className="whitespace-pre-wrap break-words text-xs">{JSON.stringify(ev.payload ?? {}, null, 2)}</pre>
        </li>
      ))}
    </ul>
  );
}

function ArtifactsList({ items }: { items: ArtifactRow[] }) {
  if (!items.length) return <div className="rounded-md bg-bg-1 p-3 shadow-card text-text-dim">No artifacts yet.</div>;
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

function ToolsStatus({ items }: { items: ToolStatus[] }) {
  if (!items.length) return <div className="rounded-md bg-bg-1 p-3 shadow-card text-text-dim">No tool status available.</div>;
  return (
    <ul className="space-y-2">
      {items.map((t, i) => (
        <li key={`${t.server}/${t.tool}/${i}`} className="rounded-md bg-bg-1 p-3 shadow-card">
          <div className="text-sm text-text">{t.server}/{t.tool}</div>
          <div className="text-xs text-text-dim">{t.status}</div>
        </li>
      ))}
    </ul>
  );
}

function ReplanQuick({ id }: { id: string }) {
  const qc = useQueryClient();
  const [open, setOpen] = useState(false);
  const [md, setMd] = useState('# Plan\n- step 1');
  const [busy, setBusy] = useState(false);
  if (!open) return <button className="rounded-md bg-surface px-3 py-1 text-sm hover:bg-bg-1" onClick={() => setOpen(true)}>Replan</button>;
  return (
    <div className="rounded-md bg-bg-1 p-3 shadow-card">
      <div className="mb-2 text-sm font-medium">Replan (Markdown)</div>
      <textarea className="h-40 w-full resize-none rounded-md bg-surface p-2 text-sm" value={md} onChange={(e) => setMd(e.target.value)} />
      <div className="mt-2 flex items-center gap-2">
        <button className="rounded-md bg-accent-500 px-3 py-1 text-sm text-black hover:bg-accent-600 disabled:opacity-50" disabled={busy} onClick={async () => { setBusy(true); try { await api.agentReplan(id, md); await qc.invalidateQueries({ queryKey: ['agent', id] }); } finally { setBusy(false); setOpen(false); } }}>Save</button>
        <button className="rounded-md bg-surface px-3 py-1 text-sm hover:bg-bg-1" onClick={() => setOpen(false)}>Cancel</button>
      </div>
    </div>
  );
}
