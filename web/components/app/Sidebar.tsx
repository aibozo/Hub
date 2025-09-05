"use client";
import { usePathname } from 'next/navigation';
import { useEffect } from 'react';
import { SessionList } from '@/components/chat/SessionList';
import { AgentList } from '@/components/agents/AgentList';
import { useUi } from '@/lib/store';

export function Sidebar() {
  const pathname = usePathname();
  const selected = useUi((s) => s.selectedChatId);
  const setSelected = useUi((s) => s.setSelectedChatId);
  const selectedAgentId = useUi((s) => s.selectedAgentId);
  const setSelectedAgentId = useUi((s) => s.setSelectedAgentId);
  // reset selection when navigating to different route
  useEffect(() => {
    if (pathname !== '/' && selected) setSelected(null);
  }, [pathname, selected, setSelected]);

  return (
    <aside className="h-[calc(100dvh-56px)] border-r border-border bg-bg p-3 text-text-dim">
      {pathname === '/' && (
        <SessionList selected={selected} onSelect={(id) => setSelected(id)} />
      )}
      {pathname === '/agents' && (
        <AgentList selected={selectedAgentId} onSelect={setSelectedAgentId} />
      )}
      {pathname !== '/' && pathname !== '/agents' && (
        <div className="rounded-md bg-bg-1 p-3 shadow-card">Contextual sidebar</div>
      )}
    </aside>
  );
}
