-- 0003: Agents schema (Agent, AgentIssue) and links

-- Agent table with default id if not provided
CREATE TABLE IF NOT EXISTS Agent (
  id TEXT PRIMARY KEY NOT NULL DEFAULT (lower(hex(randomblob(16)))),
  task_id INTEGER NOT NULL,
  title TEXT NOT NULL,
  status TEXT NOT NULL,
  plan_artifact_id INTEGER NULL,
  root_dir TEXT NOT NULL,
  model TEXT NULL,
  servers_json TEXT NULL,
  auto_approval_level INTEGER NOT NULL DEFAULT 1,
  created_at DATETIME NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  updated_at DATETIME NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  FOREIGN KEY(task_id) REFERENCES Task(id) ON DELETE CASCADE,
  FOREIGN KEY(plan_artifact_id) REFERENCES Artifact(id) ON DELETE SET NULL
);

CREATE INDEX IF NOT EXISTS idx_agent_status ON Agent(status, updated_at DESC);

CREATE TRIGGER IF NOT EXISTS trg_agent_updated_at
AFTER UPDATE ON Agent FOR EACH ROW
BEGIN
  UPDATE Agent SET updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE id = OLD.id;
END;

-- AgentIssue table
CREATE TABLE IF NOT EXISTS AgentIssue (
  id TEXT PRIMARY KEY NOT NULL DEFAULT (lower(hex(randomblob(16)))),
  agent_id TEXT NOT NULL,
  severity TEXT NOT NULL,
  title TEXT NOT NULL,
  details_md TEXT NULL,
  action_required INTEGER NOT NULL DEFAULT 0,
  ts DATETIME NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  FOREIGN KEY(agent_id) REFERENCES Agent(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_agentissue_agent_ts ON AgentIssue(agent_id, ts DESC);

-- Link Event and Artifact to Agent
ALTER TABLE Event ADD COLUMN agent_id TEXT NULL;
ALTER TABLE Artifact ADD COLUMN agent_id TEXT NULL;

CREATE INDEX IF NOT EXISTS idx_event_agent ON Event(agent_id, id DESC);
CREATE INDEX IF NOT EXISTS idx_artifact_agent ON Artifact(agent_id, id DESC);

