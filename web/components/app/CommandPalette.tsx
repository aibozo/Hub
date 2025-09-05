"use client";
import { useEffect, useMemo, useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { api } from '@/lib/api';
import { useUi } from '@/lib/store';
import { useRouter } from 'next/navigation';

type Item = { kind: 'session' | 'agent' | 'task' | 'action'; id?: string; title: string; onRun?: () => void };

export function CommandPalette() {
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState('');
  const router = useRouter();
  const setSelectedChatId = useUi((s) => s.setSelectedChatId);
  const setSelectedAgentId = useUi((s) => s.setSelectedAgentId);
  const toggleRight = useUi((s) => s.toggleRight);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'k') { e.preventDefault(); setOpen((v) => !v); }
      if (e.key === 'Escape') setOpen(false);
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, []);

  const { data: sessions } = useQuery({ queryKey: ['sessions'], queryFn: api.listSessions, staleTime: 15_000, enabled: open });
  const { data: agents } = useQuery({ queryKey: ['agents'], queryFn: api.listAgents, staleTime: 30_000, enabled: open });
  const { data: tasks } = useQuery({ queryKey: ['tasks'], queryFn: api.listTasks, staleTime: 10_000, enabled: open });

  const items: Item[] = useMemo(() => {
    const acts: Item[] = [
      { kind: 'action', title: 'New chat', onRun: async () => { const s = await api.createSession(); setSelectedChatId(s.id); router.push('/'); } },
      { kind: 'action', title: 'New agent', onRun: () => router.push('/agents') },
      { kind: 'action', title: 'New task', onRun: () => router.push('/research') },
      { kind: 'action', title: 'Toggle Activity', onRun: () => toggleRight() },
    ];
    const ss = (sessions ?? []).map((s) => ({ kind: 'session' as const, id: s.id, title: s.title || s.id }));
    const as = (agents ?? []).map((a) => ({ kind: 'agent' as const, id: a.id, title: a.title }));
    const ts = (tasks ?? []).map((t) => ({ kind: 'task' as const, id: String(t.id), title: t.title }));
    return [...acts, ...ss, ...as, ...ts];
  }, [sessions, agents, tasks]);

  const filtered = items.filter((i) => i.title.toLowerCase().includes(query.toLowerCase()));

  if (!open) return null;
  return (
    <div className="fixed inset-0 z-40 bg-black/40" onClick={() => setOpen(false)}>
      <div className="mx-auto mt-24 w-full max-w-xl rounded-md bg-bg shadow-pop" onClick={(e) => e.stopPropagation()} role="dialog" aria-modal="true">
        <div className="border-b border-border p-2">
          <input
            autoFocus
            aria-label="Command palette"
            className="w-full rounded-md bg-surface px-3 py-2 text-sm outline-none focus:ring-2 focus:ring-[color:var(--accent-500)]"
            placeholder="Search sessions, agents, tasks…"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            onKeyDown={(e) => { if (e.key === 'Enter' && filtered[0]) runItem(filtered[0]); }}
          />
        </div>
        <ul className="max-h-80 overflow-y-auto p-2">
          {filtered.map((i, idx) => (
            <li key={`${i.kind}:${i.id ?? idx}`}>
              <button className="w-full rounded-md px-3 py-2 text-left text-sm hover:bg-bg-1" onClick={() => runItem(i)}>
                <span className="text-text">{i.title}</span>
                <span className="ml-2 text-2xs text-text-dim">{i.kind}</span>
              </button>
            </li>
          ))}
          {!filtered.length && <li className="px-3 py-2 text-sm text-text-dim">No matches.</li>}
        </ul>
        <div className="border-t border-border p-2 text-2xs text-text-dim">Press Esc to close • Enter to run</div>
      </div>
    </div>
  );

  function runItem(i: Item) {
    if (i.kind === 'session' && i.id) { setSelectedChatId(i.id); setOpen(false); router.push('/'); return; }
    if (i.kind === 'agent' && i.id) { useUi.getState().setSelectedAgentId(i.id); setOpen(false); router.push('/agents'); return; }
    if (i.kind === 'task' && i.id) { setOpen(false); router.push('/research'); return; }
    if (i.kind === 'action' && i.onRun) { i.onRun(); setOpen(false); return; }
  }
}

