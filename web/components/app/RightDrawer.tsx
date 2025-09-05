"use client";
import { useUi } from '@/lib/store';

export function RightDrawer() {
  const { rightOpen, toggleRight } = useUi();
  return (
    <div className={`fixed right-0 top-14 z-20 h-[calc(100dvh-56px)] w-[360px] transform border-l border-border bg-bg-2 shadow-pop transition-transform duration-300 ${rightOpen ? 'translate-x-0' : 'translate-x-full'}`}>
      <div className="flex items-center justify-between border-b border-border p-3">
        <div className="text-sm font-medium">Activity</div>
        <button className="text-text-dim hover:text-text" onClick={toggleRight}>Close</button>
      </div>
      <div className="h-full overflow-y-auto p-3 text-text-dim">
        <ActivityList />
      </div>
    </div>
  );
}

function ActivityList() {
  const items = useUi((s) => s.activity);
  if (!items.length) {
    return <div className="rounded-md bg-bg-1 p-3 shadow-card">No activity yet.</div>;
  }
  return (
    <ul className="space-y-2">
      {items.map((it, i) => (
        <li key={i} className="rounded-md bg-bg-1 p-3 shadow-card">
          <div className="mb-1 text-xs text-text-dim">{new Date(it.ts).toLocaleTimeString()} · {it.kind}</div>
          <pre className="whitespace-pre-wrap break-words text-xs">{JSON.stringify(it.payload, null, 2)}</pre>
        </li>
      ))}
    </ul>
  );
}
