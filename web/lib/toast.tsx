"use client";
import { create } from 'zustand';
import { useEffect } from 'react';

export type Toast = { id: number; kind: 'info' | 'error' | 'success'; text: string };

type ToastState = {
  items: Toast[];
  push: (t: Omit<Toast, 'id'>) => void;
  dismiss: (id: number) => void;
};

let nextId = 1;
export const useToast = create<ToastState>((set) => ({
  items: [],
  push: (t) => set((s) => ({ items: [...s.items, { ...t, id: nextId++ }] })),
  dismiss: (id) => set((s) => ({ items: s.items.filter((x) => x.id !== id) })),
}));

export function Toaster() {
  const items = useToast((s) => s.items);
  const dismiss = useToast((s) => s.dismiss);
  useEffect(() => {
    const timers = items.map((t) => setTimeout(() => dismiss(t.id), 4000));
    return () => timers.forEach(clearTimeout);
  }, [items, dismiss]);
  return (
    <div className="pointer-events-none fixed bottom-4 right-4 z-50 space-y-2">
      {items.map((t) => (
        <div key={t.id} className={`pointer-events-auto rounded-md px-3 py-2 text-sm shadow-pop ${cls(t.kind)}`}>{t.text}</div>
      ))}
    </div>
  );
}

function cls(kind: Toast['kind']) {
  if (kind === 'error') return 'bg-[color:rgba(255,107,107,0.15)] text-[color:var(--err)]';
  if (kind === 'success') return 'bg-[color:rgba(25,195,125,0.15)] text-[color:var(--ok)]';
  return 'bg-bg-1 text-text-dim';
}

