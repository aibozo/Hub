"use client";
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { api } from '@/lib/api';
import type { AgentRow } from '@/lib/types';
import { useState } from 'react';

export function AgentList({ selected, onSelect }: { selected?: string | null; onSelect: (id: string) => void }) {
  const { data, isLoading, isError } = useQuery({ queryKey: ['agents'], queryFn: api.listAgents, staleTime: 30_000 });
  return (
    <div className="flex h-full flex-col gap-2">
      <NewAgentInline onCreated={(id) => onSelect(id)} />
      <div className="flex-1 overflow-y-auto">
        {isLoading && <div className="text-text-dim">Loading…</div>}
        {isError && <div className="text-[color:var(--err)]">Failed to load agents</div>}
        <ul className="mt-2 space-y-1">
          {(data ?? []).map((a) => (
            <li key={a.id}>
              <button className={`w-full rounded-md px-3 py-2 text-left text-sm hover:bg-bg-1 ${selected === a.id ? 'bg-bg-1' : ''}`} onClick={() => onSelect(a.id)}>
                <div className="flex items-center justify-between">
                  <div className="truncate text-text">{a.title}</div>
                  <span className="rounded-full bg-surface px-2 py-0.5 text-2xs text-text-dim">{a.status}</span>
                </div>
                <div className="text-2xs truncate text-text-dim">{a.model ?? 'model: default'} · {new Date(a.updated_at).toLocaleString()}</div>
              </button>
            </li>
          ))}
        </ul>
      </div>
    </div>
  );
}

function NewAgentInline({ onCreated }: { onCreated: (id: string) => void }) {
  const qc = useQueryClient();
  const [open, setOpen] = useState(false);
  const [title, setTitle] = useState('New Agent');
  const [root, setRoot] = useState('dev/agent');
  const [model, setModel] = useState('');
  const [busy, setBusy] = useState(false);
  if (!open) return <button className="rounded-md bg-accent-500 px-3 py-2 text-sm text-black hover:bg-accent-600" onClick={() => setOpen(true)}>New Agent</button>;
  return (
    <div className="rounded-md bg-bg-1 p-3 shadow-card">
      <div className="mb-2 text-sm font-medium">Create Agent</div>
      <div className="mb-2 grid grid-cols-1 gap-2">
        <input className="rounded-md bg-surface px-2 py-1 text-sm" placeholder="Title" value={title} onChange={(e) => setTitle(e.target.value)} />
        <input className="rounded-md bg-surface px-2 py-1 text-sm" placeholder="Root dir (relative to storage)" value={root} onChange={(e) => setRoot(e.target.value)} />
        <input className="rounded-md bg-surface px-2 py-1 text-sm" placeholder="Model (optional)" value={model} onChange={(e) => setModel(e.target.value)} />
      </div>
      <div className="flex items-center gap-2">
        <button
          className="rounded-md bg-accent-500 px-3 py-1 text-sm text-black hover:bg-accent-600 disabled:opacity-50"
          disabled={busy}
          onClick={async () => {
            setBusy(true);
            try {
              const task = await api.createTask(`Agent: ${title}`, 'open');
              const agent: any = await api.createAgent({ task_id: (task as any).id ?? task['id'] ?? 0, title, root_dir: root, model: model || undefined, auto_approval_level: 2 });
              await qc.invalidateQueries({ queryKey: ['agents'] });
              onCreated(agent.id);
              setOpen(false);
            } catch (e) {
              const { useToast } = await import('@/lib/toast');
              useToast.getState().push({ kind: 'error', text: `Failed to create agent: ${String(e)}` });
            } finally { setBusy(false); }
          }}
        >
          Create
        </button>
        <button className="rounded-md bg-surface px-3 py-1 text-sm hover:bg-bg-1" onClick={() => setOpen(false)}>Cancel</button>
      </div>
    </div>
  );
}
