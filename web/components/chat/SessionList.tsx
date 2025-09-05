"use client";
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { api } from '@/lib/api';

export function SessionList({ selected, onSelect, onCreate }: { selected?: string | null; onSelect: (id: string) => void; onCreate?: () => void }) {
  const qc = useQueryClient();
  const { data, isLoading, isError } = useQuery({ queryKey: ['sessions'], queryFn: api.listSessions, staleTime: 15_000 });

  return (
    <div className="flex h-full flex-col gap-2">
      <button
        className="rounded-md bg-accent-500 px-3 py-2 text-sm text-black hover:bg-accent-600"
        onClick={async () => {
          const s = await api.createSession();
          await qc.invalidateQueries({ queryKey: ['sessions'] });
          onSelect(s.id);
          onCreate?.();
        }}
      >
        New Chat
      </button>
      <div className="flex-1 overflow-y-auto">
        {isLoading && <div className="text-text-dim">Loading…</div>}
        {isError && <div className="text-[color:var(--err)]">Failed to load sessions</div>}
        <ul className="mt-2 space-y-1">
          {(data ?? []).map((s) => (
            <li key={s.id}>
              <button
                className={`w-full rounded-md px-3 py-2 text-left text-sm hover:bg-bg-1 ${selected === s.id ? 'bg-bg-1' : ''}`}
                onClick={() => onSelect(s.id)}
              >
                <div className="truncate text-text">{s.title || s.id}</div>
                <div className="text-2xs truncate text-text-dim">{new Date(s.updated_at).toLocaleString()}</div>
              </button>
            </li>
          ))}
        </ul>
      </div>
    </div>
  );
}
