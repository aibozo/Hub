"use client";
import { useState } from 'react';

export function Composer({ onSend, disabled }: { onSend: (text: string) => void; disabled?: boolean }) {
  const [v, setV] = useState('');
  const send = () => {
    const t = v.trim();
    if (!t) return;
    onSend(t);
    setV('');
  };
  return (
    <div className="border-t border-border bg-bg p-3">
      <textarea
        value={v}
        onChange={(e) => setV(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === 'Enter' && !e.shiftKey) {
            e.preventDefault();
            send();
          }
        }}
        rows={2}
        aria-label="Chat composer"
        className="w-full resize-none rounded-md bg-bg-1 p-3 outline-none focus:ring-2 focus:ring-[color:var(--accent-500)]"
        placeholder="Message the agent…"
        disabled={!!disabled}
      />
      <div className="mt-2 flex items-center justify-end gap-2">
        <button className="rounded-md bg-surface px-3 py-1 text-sm hover:bg-bg-1" onClick={send} disabled={!!disabled}>Send</button>
      </div>
    </div>
  );
}

