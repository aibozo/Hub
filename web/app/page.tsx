"use client";
import { HealthBadge } from '@/components/app/HealthBadge';
import { ChatView } from '@/components/chat/ChatView';
import { useUi } from '@/lib/store';

export default function Page() {
  const selected = useUi((s) => s.selectedChatId);
  return (
    <div className="h-full p-6">
      <div className="mb-4 flex items-center gap-4">
        <h1 className="text-xl font-semibold">Chat</h1>
        <HealthBadge />
      </div>
      {selected ? (
        <div className="flex h-[calc(100dvh-56px-80px)] min-h-[400px] flex-col rounded-md bg-bg shadow-card">
          <ChatView sessionId={selected} />
        </div>
      ) : (
        <div className="rounded-md bg-bg-1 p-6 shadow-card text-text-dim">
          Select a session or create a new chat to begin.
        </div>
      )}
    </div>
  );
}
