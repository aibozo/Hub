"use client";
import { useState } from 'react';
import type { EphemeralApproval } from '@/lib/types';

export function ApprovalBanner({ prompt, onApprove, onDeny, onExplain }: {
  prompt: EphemeralApproval;
  onApprove: () => Promise<void> | void;
  onDeny: () => Promise<void> | void;
  onExplain?: () => Promise<any> | void;
}) {
  const [busy, setBusy] = useState(false);
  const { title, action } = prompt;
  return (
    <div className="mx-auto mb-2 max-w-3xl rounded-md border border-[color:var(--warn)] bg-[color:rgba(244,191,80,0.08)] p-3 text-sm">
      <div className="mb-2 font-medium text-[color:var(--warn)]">Approval required</div>
      <div className="mb-2 text-text">
        <div className="font-medium">{title}</div>
        <div className="text-text-dim">{action.command}{action.writes ? ' (writes)' : ''}</div>
        {action.paths && action.paths.length > 0 && (
          <div className="mt-1 truncate text-xs text-text-dim">paths: {action.paths.join(', ')}</div>
        )}
        {action.intent && <div className="mt-1 text-xs text-text-dim">intent: {action.intent}</div>}
      </div>
      <div className="flex items-center gap-2">
        <button
          aria-label="Approve request"
          disabled={busy}
          className="rounded-md bg-[color:var(--ok)] px-3 py-1 text-sm text-black hover:opacity-90 disabled:opacity-50"
          onClick={async () => { setBusy(true); try { await onApprove(); } finally { setBusy(false); } }}
        >
          Approve
        </button>
        <button
          aria-label="Deny request"
          disabled={busy}
          className="rounded-md bg-[color:var(--err)] px-3 py-1 text-sm text-black hover:opacity-90 disabled:opacity-50"
          onClick={async () => { setBusy(true); try { await onDeny(); } finally { setBusy(false); } }}
        >
          Deny
        </button>
        {onExplain && (
          <button className="rounded-md bg-surface px-3 py-1 text-sm text-text hover:bg-bg-1" onClick={() => onExplain?.()}>Explain</button>
        )}
      </div>
    </div>
  );
}

