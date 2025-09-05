"use client";
import { useMemo, useState } from 'react';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { api } from '@/lib/api';
import type { Task } from '@/lib/types';

export function TaskTable({ onOpen }: { onOpen: (task: Task) => void }) {
  const { data, isLoading, isError } = useQuery({ queryKey: ['tasks'], queryFn: api.listTasks, staleTime: 10_000 });
  const [filter, setFilter] = useState<'all' | 'open' | 'in-progress' | 'done' | 'blocked'>('all');
  const [q, setQ] = useState('');

  const tasks = useMemo(() => {
    let t = (data ?? []).slice();
    if (filter !== 'all') t = t.filter((x) => x.status.toLowerCase() === filter);
    if (q.trim()) {
      const s = q.toLowerCase();
      t = t.filter((x) => x.title.toLowerCase().includes(s) || (x.tags ?? '').toLowerCase().includes(s));
    }
    return t;
  }, [data, filter, q]);

  const isEmpty = !isLoading && !isError && (tasks.length === 0);
  return (
    <div className="flex h-full flex-col">
      <div className="mb-3 flex items-center gap-2">
        <FilterChip label="All" active={filter==='all'} onClick={() => setFilter('all')} />
        <FilterChip label="Open" active={filter==='open'} onClick={() => setFilter('open')} />
        <FilterChip label="In‑Progress" active={filter==='in-progress'} onClick={() => setFilter('in-progress')} />
        <FilterChip label="Done" active={filter==='done'} onClick={() => setFilter('done')} />
        <FilterChip label="Blocked" active={filter==='blocked'} onClick={() => setFilter('blocked')} />
        <div className="ml-auto" />
        <input className="rounded-md bg-surface px-2 py-1 text-sm" placeholder="Search…" value={q} onChange={(e) => setQ(e.target.value)} />
      </div>
      <div className="flex-1 overflow-auto">
        {isEmpty && (
          <div className="rounded-md bg-bg-1 p-6 text-text-dim">No tasks yet — create your first task to start a brief.</div>
        )}
        {isLoading && <div className="text-text-dim">Loading…</div>}
        {isError && <div className="text-[color:var(--err)]">Failed to load tasks</div>}
        {!isEmpty && (
        <table className="w-full text-sm">
          <thead className="sticky top-0 bg-bg">
            <tr className="text-left text-text-dim">
              <th className="px-2 py-2">Title</th>
              <th className="px-2 py-2">Status</th>
              <th className="px-2 py-2">Tags</th>
              <th className="px-2 py-2">Updated</th>
            </tr>
          </thead>
          <tbody>
            {tasks.map((t) => (
              <tr key={t.id} className="cursor-pointer border-t border-border hover:bg-bg-1" onClick={() => onOpen(t)}>
                <td className="px-2 py-2 text-text">{t.title}</td>
                <td className="px-2 py-2"><StatusBadge status={t.status} /></td>
                <td className="px-2 py-2 text-text-dim">{t.tags ?? ''}</td>
                <td className="px-2 py-2 text-text-dim">{new Date(t.updated_at).toLocaleString()}</td>
              </tr>
            ))}
          </tbody>
        </table>
        )}
      </div>
    </div>
  );
}

function FilterChip({ label, active, onClick }: { label: string; active?: boolean; onClick: () => void }) {
  return (
    <button onClick={onClick} className={`rounded-full px-2 py-1 text-xs ${active ? 'bg-accent-500 text-black' : 'bg-surface text-text-dim hover:bg-bg-1'}`}>{label}</button>
  );
}

function StatusBadge({ status }: { status: string }) {
  const s = status.toLowerCase();
  const color = s === 'done' ? 'bg-[color:var(--ok)] text-black' : s === 'blocked' ? 'bg-[color:var(--err)] text-black' : 'bg-surface text-text-dim';
  return <span className={`rounded-full px-2 py-0.5 text-2xs ${color}`}>{status}</span>;
}
