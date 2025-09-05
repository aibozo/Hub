"use client";
import { useState } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { TaskTable } from '@/components/research/TaskTable';
import { TaskDrawer } from '@/components/research/TaskDrawer';
import type { Task } from '@/lib/types';
import { api } from '@/lib/api';

export default function ResearchPage() {
  const qc = useQueryClient();
  const [open, setOpen] = useState<Task | null>(null);
  const [newOpen, setNewOpen] = useState(false);
  const [title, setTitle] = useState('New Task');
  const [busy, setBusy] = useState(false);
  return (
    <div className="h-full p-6">
      <div className="mb-4 flex items-center justify-between">
        <h1 className="text-xl font-semibold">Research</h1>
        {!newOpen ? (
          <button className="rounded-md bg-accent-500 px-3 py-2 text-sm text-black hover:bg-accent-600" onClick={() => setNewOpen(true)}>New Task</button>
        ) : (
          <div className="flex items-center gap-2">
            <input className="rounded-md bg-surface px-2 py-1 text-sm" placeholder="Task title" value={title} onChange={(e) => setTitle(e.target.value)} />
            <button className="rounded-md bg-accent-500 px-3 py-1 text-sm text-black hover:bg-accent-600 disabled:opacity-50" disabled={busy} onClick={async () => { setBusy(true); try { await api.createTask(title, 'open'); await qc.invalidateQueries({ queryKey: ['tasks'] }); setNewOpen(false); } catch (e) { const { useToast } = await import('@/lib/toast'); useToast.getState().push({ kind: 'error', text: `Failed to create task: ${String(e)}` }); } finally { setBusy(false); } }}>Create</button>
            <button className="rounded-md bg-surface px-3 py-1 text-sm hover:bg-bg-1" onClick={() => setNewOpen(false)}>Cancel</button>
          </div>
        )}
      </div>
      <div className="flex h-[calc(100dvh-56px-80px)] min-h-[400px] flex-col rounded-md bg-bg shadow-card">
        <div className="flex-1 overflow-hidden p-4">
          <TaskTable onOpen={(t) => setOpen(t)} />
        </div>
      </div>
      {open && <TaskDrawer task={open} onClose={() => setOpen(null)} />}
    </div>
  );
}
