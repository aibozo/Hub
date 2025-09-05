export const API_BASE = process.env.NEXT_PUBLIC_API_BASE ?? 'http://127.0.0.1:6061';

export async function json<T>(path: string, init?: RequestInit): Promise<T> {
  const res = await fetch(`${API_BASE}${path}`, {
    ...init,
    headers: { 'content-type': 'application/json', ...(init?.headers || {}) },
  });
  if (!res.ok) throw new Error(`${res.status} ${res.statusText}`);
  return res.json() as Promise<T>;
}

export const api = {
  ready: async (): Promise<boolean> => {
    try {
      const res = await fetch(`${API_BASE}/ready`, { cache: 'no-store' });
      return res.ok;
    } catch (_) {
      return false;
    }
  },
  approvalPrompt: async (): Promise<import('./types').EphemeralApproval | null> => {
    const res = await fetch(`${API_BASE}/api/approval/prompt`, { cache: 'no-store' });
    if (res.status === 204) return null;
    if (!res.ok) throw new Error(`${res.status} ${res.statusText}`);
    return res.json();
  },
  // Approval answer returns 200 with empty body
  answerApproval: (id: string, answer: 'approve' | 'deny') =>
    fetch(`${API_BASE}/api/approval/answer`, {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify({ id, answer }),
    }).then(r => { if (!r.ok) throw new Error(`${r.status} ${r.statusText}`); }),
  explainApproval: (id: string) => json(`/api/approval/explain/${id}`),
  listAgents: () => json<import('./types').AgentRow[]>(`/api/agents`),
  getAgent: (id: string) => json<import('./types').AgentSnapshot>(`/api/agents/${id}`),
  agentPause: (id: string) => fetch(`${API_BASE}/api/agents/${id}/pause`, { method: 'POST' }).then(r => { if (!r.ok) throw new Error(`${r.status}`); }),
  agentResume: (id: string) => fetch(`${API_BASE}/api/agents/${id}/resume`, { method: 'POST' }).then(r => { if (!r.ok) throw new Error(`${r.status}`); }),
  agentAbort: (id: string) => fetch(`${API_BASE}/api/agents/${id}/abort`, { method: 'POST' }).then(r => { if (!r.ok) throw new Error(`${r.status}`); }),
  // Replan returns 200 with empty body
  agentReplan: (id: string, content_md: string) =>
    fetch(`${API_BASE}/api/agents/${id}/replan`, {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify({ content_md }),
    }).then(r => { if (!r.ok) throw new Error(`${r.status} ${r.statusText}`); }),
  agentArtifacts: (id: string) => json<import('./types').ArtifactRow[]>(`/api/agents/${id}/artifacts`),
  toolsStatus: () => json<import('./types').ToolStatus[]>(`/api/tools/status`),
  listTasks: () => json<import('./types').Task[]>(`/api/tasks`),
  createTask: (title: string, status: string = 'open', tags?: string) => json(`/api/tasks`, { method: 'POST', body: JSON.stringify({ title, status, tags }) }),
  taskAtoms: (id: number) => json(`/api/tasks/${id}/atoms`),
  taskArtifacts: (id: number) => json<import('./types').ArtifactRow[]>(`/api/tasks/${id}/artifacts`),
  contextPack: (task_id: number, token_budget = 2048, k_cards = 12) => json(`/api/context/pack`, { method: 'POST', body: JSON.stringify({ task_id, token_budget, k_cards }) }),
  contextExpand: (handle: string, depth?: number) => json(`/api/context/expand`, { method: 'POST', body: JSON.stringify({ handle, depth }) }),
  // Run brief returns 200 with empty body
  runBrief: (job: 'arxiv' | 'news') =>
    fetch(`${API_BASE}/api/schedules/run/${job}`, { method: 'POST' })
      .then(r => { if (!r.ok) throw new Error(`${r.status} ${r.statusText}`); }),
  createAgent: (args: { task_id: number; title: string; root_dir: string; model?: string; auto_approval_level?: number; servers?: string[] }) =>
    json(`/api/agents`, { method: 'POST', body: JSON.stringify(args) }),
  listSessions: () => json<Array<{ id: string; updated_at: string; title?: string }>>(`/api/chat/sessions`),
  createSession: () => json<{ id: string; messages: Array<{ id: string; role: string; content: string; at?: string }> }>(`/api/chat/sessions`, { method: 'POST' }),
  getSession: (id: string) => json<{ id: string; messages: Array<{ id: string; role: string; content: string; at?: string }> }>(`/api/chat/sessions/${id}`),
  deleteSession: (id: string) => fetch(`${API_BASE}/api/chat/sessions/${id}`, { method: 'DELETE' }).then(r => { if (!r.ok) throw new Error(`${r.status}`); }),
  // Append returns JSON { id, role, content, at }
  appendMessage: (id: string, msg: { role: string; content: string }) =>
    json<{ id: string; role: string; content: string; at?: string }>(`/api/chat/sessions/${id}/append`, { method: 'POST', body: JSON.stringify(msg) }),
  streamChat: (body: { session_id?: string; messages: Array<{ role: string; content: string }>; model?: string; max_steps?: number }, signal?: AbortSignal) =>
    fetch(`${API_BASE}/api/chat/stream`, { method: 'POST', body: JSON.stringify(body), signal, headers: { 'content-type': 'application/json' } }),
};
