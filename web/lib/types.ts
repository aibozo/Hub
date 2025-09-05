export type ChatMessage = { id: string; role: 'system' | 'user' | 'assistant' | 'tool'; content: string; at?: string };
export type ChatSession = { id: string; messages: ChatMessage[] };

export type ProposedAction = {
  command: string;
  writes?: boolean;
  paths?: string[];
  intent?: string | null;
};

export type EphemeralApproval = {
  id: string;
  title: string;
  action: ProposedAction;
  details?: any;
};

export type AgentRow = {
  id: string;
  task_id: number;
  title: string;
  status: string;
  plan_artifact_id?: number | null;
  root_dir: string;
  model?: string | null;
  auto_approval_level: number;
  created_at: string;
  updated_at: string;
};

export type AgentSnapshot = { agent: AgentRow; events: Array<{ id: number; kind: string; ts: string; payload?: any }> };

export type ArtifactRow = { id: number; path: string; mime?: string | null; bytes?: number | null; origin_url?: string | null };

export type ToolStatus = { server: string; tool: string; status: string };
export type Task = { id: number; title: string; status: string; tags?: string | null; created_at: string; updated_at: string };
