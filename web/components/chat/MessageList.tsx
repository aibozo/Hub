"use client";
import { useEffect, useRef } from 'react';
import type { ChatMessage } from '@/lib/types';
import { parseFences } from '@/lib/markdown';
import { CodeBlock } from '@/components/primitives/CodeBlock';

export function MessageList({ messages, toolInlines, streamingId }: { messages: ChatMessage[]; toolInlines?: string[]; streamingId?: string | null }) {
  const endRef = useRef<HTMLDivElement | null>(null);
  useEffect(() => { endRef.current?.scrollIntoView({ behavior: 'smooth', block: 'end' }); }, [messages, streamingId]);
  return (
    <div className="flex-1 overflow-y-auto p-4">
      <div className="mx-auto flex max-w-3xl flex-col gap-3">
        {messages.map((m) => (
          <Bubble key={m.id} role={m.role} content={m.content} streaming={streamingId === m.id} />
        ))}
        {toolInlines?.map((t, i) => (<Bubble key={`tool-${i}`} role="tool" content={t} />))}
        <div ref={endRef} />
      </div>
    </div>
  );
}

function Bubble({ role, content, streaming }: { role: string; content: string; streaming?: boolean }) {
  const isUser = role === 'user';
  const segs = parseFences(content);
  return (
    <div className={`flex ${isUser ? 'justify-end' : 'justify-start'}`}>
      <div className={`max-w-[720px] rounded-lg p-3 shadow-card ${isUser ? 'bg-bg-1' : 'bg-bg-1'}`}>
        {segs.map((s, i) => s.type === 'code' ? (
          <div key={i} className="mb-2"><CodeBlock value={s.content} language={s.lang} /></div>
        ) : (
          <p key={i} className="whitespace-pre-wrap">{s.content}{streaming && i === segs.length - 1 ? '▍' : ''}</p>
        ))}
      </div>
    </div>
  );
}
