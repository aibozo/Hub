"use client";
import { useEffect, useRef, useState } from 'react';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { api } from '@/lib/api';
import type { ChatMessage, ChatSession } from '@/lib/types';
import { readSSE, type ChatEvent } from '@/lib/sse';
import { Composer } from './Composer';
import { MessageList } from './MessageList';
import { useUi } from '@/lib/store';
import { ApprovalBanner } from '@/components/approvals/ApprovalBanner';
import { useThemeSettings } from '@/components/app/ThemeProvider';

export function ChatView({ sessionId }: { sessionId: string }) {
  const qc = useQueryClient();
  const { data: session } = useQuery({ queryKey: ['session', sessionId], queryFn: () => api.getSession(sessionId), refetchOnWindowFocus: false });
  const [pending, setPending] = useState('');
  const [streaming, setStreaming] = useState(false);
  const [toolInlines, setToolInlines] = useState<string[]>([]);
  const [finalizing, setFinalizing] = useState(false);
  const [localMessages, setLocalMessages] = useState<ChatMessage[]>([]);
  const [streamingId, setStreamingId] = useState<string | null>(null);
  const [tokenBuffer, setTokenBuffer] = useState<string>('');
  const abortRef = useRef<AbortController | null>(null);
  const finalizeRef = useRef<Promise<void> | null>(null);
  const pushActivity = useUi((s) => s.pushActivity);
  const { settings } = useThemeSettings();

  const messages: ChatMessage[] = localMessages;

  // Seed/refresh local messages from server session with merge-by-id (never delete local-only)
  useEffect(() => {
    const serverMsgs = session?.messages ?? [];
    setLocalMessages((prev) => {
      if (!prev.length) return serverMsgs;
      const map = new Map(prev.map((m) => [m.id, m] as const));
      for (const sm of serverMsgs) {
        const pm = map.get(sm.id);
        if (pm) {
          // Prefer whichever content is longer (avoid overwriting streamed content with empty/partial server copies)
          const content = (sm.content?.length ?? 0) >= (pm.content?.length ?? 0) ? sm.content : pm.content;
          map.set(sm.id, { ...pm, role: sm.role, content, at: sm.at ?? pm.at });
        } else {
          map.set(sm.id, sm as any);
        }
      }
      return Array.from(map.values());
    });
  }, [sessionId, session?.messages]);
  const { data: approvalPrompt, refetch: refetchApproval } = useQuery({
    queryKey: ['approval', 'prompt'],
    queryFn: api.approvalPrompt,
    refetchInterval: (q) => (q.state.data ? 3000 : 1500),
  });

  async function send(text: string) {
    // If previous turn is finishing persistence, wait for it
    if (finalizeRef.current) {
      try { await finalizeRef.current; } catch { /* ignore */ }
    }
    // If a previous stream is active, abort it before starting a new one
    if (streaming) {
      abortRef.current?.abort();
    }

    // Abort any existing stream before starting a new one
    abortRef.current?.abort();
    abortRef.current = new AbortController();

    // Append user message and update cache with server timestamp
    const userSaved = await api.appendMessage(sessionId, { role: 'user', content: text });
    qc.setQueryData(['session', sessionId], (old: any) => {
      const base = old ?? { id: sessionId, messages: [] };
      return { ...base, messages: [...(base.messages ?? []), userSaved] };
    });
    setLocalMessages((prev) => [...prev, userSaved as any]);

    // Build the message history for the next turn from local state (single source of truth)
    const toSend = localMessages.map((m) => ({ role: m.role, content: m.content }));

    setStreaming(true);
    try {
      const res = await api.streamChat({ session_id: sessionId, messages: toSend, max_steps: 6 }, abortRef.current.signal);
      for await (const evt of readSSE(res)) {
        handleEvent(evt);
      }
    } catch (e) {
      pushActivity({ ts: Date.now(), kind: 'error', payload: { error: String(e) } });
      const { useToast } = await import('@/lib/toast');
      useToast.getState().push({ kind: 'error', text: `Stream error: ${String(e)}` });
      setStreaming(false);
    }
  }

  function handleEvent(evt: ChatEvent) {
    if (evt.event === 'assistant_started') {
      const id = (evt as any).data?.id as string | undefined;
      if (id) {
        setStreamingId(id);
        // Insert placeholder message
        setLocalMessages((prev) => [...prev, { id, role: 'assistant', content: '', at: new Date().toISOString() } as any]);
        // If any tokens arrived before assistant_started, flush them now
        setTokenBuffer((buf) => {
          if (!buf) return '';
          setLocalMessages((prev) => prev.map((m) => (m.id === id ? { ...m, content: buf } : m)));
          return '';
        });
      }
    } else if (evt.event === 'token') {
      const t = evt.data?.text ?? '';
      const id = (evt as any).data?.id as string | undefined;
      if (id) {
        setLocalMessages((prev) => prev.map((m) => (m.id === id ? { ...m, content: m.content + t } : m)));
      } else {
        // Fallback: buffer until assistant_started arrives to avoid overwriting old messages
        setTokenBuffer((buf) => buf + t);
      }
    } else if (evt.event === 'tool_call' || evt.event === 'tool_calls' || evt.event === 'tool_result') {
      pushActivity({ ts: Date.now(), kind: evt.event === 'tool_result' ? 'tool_result' : 'tool_call', payload: evt.data });
      // Inline logs if enabled
      if (settings.inlineToolLogs) {
        const text = JSON.stringify(evt.data);
        setToolInlines((a) => [...a, text]);
      }
    } else if (evt.event === 'error') {
      pushActivity({ ts: Date.now(), kind: 'error', payload: evt.data });
      setStreaming(false);
      setStreamingId(null);
    } else if (evt.event === 'done') {
      setToolInlines([]);
      setStreaming(false);
      setStreamingId(null);
      setTokenBuffer('');
      // Refetch session to align persisted assistant content and merge safely
      (async () => {
        try {
          const fresh = await api.getSession(sessionId);
          qc.setQueryData(['session', sessionId], fresh);
          setLocalMessages((prev) => {
            const prevById = new Map(prev.map((p) => [p.id, p] as const));
            // Use server order, but prefer longer content between server and local for each id
            const merged = fresh.messages.map((sm: any) => {
              const pm = prevById.get(sm.id);
              const content = ((sm.content?.length ?? 0) >= (pm?.content?.length ?? 0)) ? sm.content : (pm?.content ?? sm.content);
              return { ...sm, content } as ChatMessage;
            });
            return merged;
          });
        } catch (err) {
          // Best-effort; keep local state if refresh fails
          console.warn('chat: refresh on done failed', err);
        }
      })();
    }
  }

  useEffect(() => () => abortRef.current?.abort(), []);

  return (
    <div className="flex h-full flex-col">
      {!!approvalPrompt && (
        <ApprovalBanner
          prompt={approvalPrompt}
          onApprove={async () => {
            await api.answerApproval(approvalPrompt.id, 'approve');
            pushActivity({ ts: Date.now(), kind: 'info', payload: { approval: approvalPrompt.id, answer: 'approve' } });
            refetchApproval();
          }}
          onDeny={async () => {
            await api.answerApproval(approvalPrompt.id, 'deny');
            pushActivity({ ts: Date.now(), kind: 'info', payload: { approval: approvalPrompt.id, answer: 'deny' } });
            refetchApproval();
          }}
          onExplain={async () => {
            const card = await api.explainApproval(approvalPrompt.id).catch(() => null);
            if (card) pushActivity({ ts: Date.now(), kind: 'info', payload: { explain: card } });
          }}
        />
      )}
      <MessageList messages={messages} toolInlines={toolInlines} streamingId={streamingId} />
      <Composer onSend={send} disabled={streaming || finalizing} />
    </div>
  );
}
