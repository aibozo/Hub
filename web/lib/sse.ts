export type ChatEvent =
  | { event: 'token'; data: { text?: string } }
  | { event: 'assistant_started'; data: { id: string } }
  | { event: 'tool_calls'; data: any }
  | { event: 'tool_call'; data: any }
  | { event: 'tool_result'; data: any }
  | { event: 'error'; data: { message?: string } }
  | { event: 'done'; data: {} }
  | { event: 'message'; data: any };

export async function* readSSE(res: Response): AsyncGenerator<ChatEvent, void, unknown> {
  if (!res.ok || !res.body) throw new Error(`SSE response invalid: ${res.status}`);
  const reader = res.body.getReader();
  const decoder = new TextDecoder('utf-8');
  let buf = '';
  let event = 'message';
  let data: string[] = [];
  for (;;) {
    const { value, done } = await reader.read();
    if (done) break;
    buf += decoder.decode(value, { stream: true });
    let idx: number;
    while ((idx = buf.indexOf('\n')) >= 0) {
      const line = buf.slice(0, idx).trimEnd();
      buf = buf.slice(idx + 1);
      if (!line) {
        const joined = data.join('\n');
        data = [];
        try {
          yield { event, data: joined ? JSON.parse(joined) : {} } as ChatEvent;
        } catch {
          // Some streams (e.g., token pieces) may send plain text lines
          const data = event === 'token' ? { text: joined } : { raw: joined };
          yield { event, data } as unknown as ChatEvent;
        }
        event = 'message';
        continue;
      }
      if (line.startsWith('event:')) event = line.slice(6).trim();
      else if (line.startsWith('data:')) {
        // Preserve leading spaces inside data payloads. The SSE line is either "data:<payload>" or "data: <payload>".
        let v = line.slice(5);
        if (v.startsWith(' ')) v = v.slice(1);
        data.push(v);
      }
    }
  }
}
