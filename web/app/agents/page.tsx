"use client";
import { useUi } from '@/lib/store';
import { AgentDetail } from '@/components/agents/AgentDetail';

export default function AgentsPage() {
  const selected = useUi((s) => s.selectedAgentId);
  return (
    <div className="h-full p-6">
      <h1 className="mb-4 text-xl font-semibold">Agents</h1>
      {selected ? (
        <div className="flex h-[calc(100dvh-56px-80px)] min-h-[400px] flex-col rounded-md bg-bg shadow-card">
          <div className="flex-1 overflow-y-auto p-4">
            <AgentDetail id={selected} />
          </div>
        </div>
      ) : (
        <div className="rounded-md bg-bg-1 p-6 shadow-card text-text-dim">Select or create an agent to view details.</div>
      )}
    </div>
  );
}
