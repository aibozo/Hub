-- tasks
CREATE TABLE IF NOT EXISTS Task (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  title TEXT NOT NULL,
  status TEXT NOT NULL,
  tags TEXT NULL,
  created_at DATETIME NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  updated_at DATETIME NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);

-- digests
CREATE TABLE IF NOT EXISTS TaskDigest (
  task_id INTEGER NOT NULL UNIQUE,
  short TEXT NULL,
  paragraph TEXT NULL,
  tokens INTEGER NULL,
  FOREIGN KEY(task_id) REFERENCES Task(id) ON DELETE CASCADE
);

-- atoms
CREATE TABLE IF NOT EXISTS Atom (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  task_id INTEGER NOT NULL,
  kind TEXT NOT NULL,
  text TEXT NOT NULL,
  tags TEXT NULL,
  created_at DATETIME NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  FOREIGN KEY(task_id) REFERENCES Task(id) ON DELETE CASCADE
);

-- artifacts
CREATE TABLE IF NOT EXISTS Artifact (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  task_id INTEGER NOT NULL,
  path TEXT NOT NULL,
  mime TEXT NULL,
  sha256 TEXT NULL,
  created_at DATETIME NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  FOREIGN KEY(task_id) REFERENCES Task(id) ON DELETE CASCADE
);

-- events
CREATE TABLE IF NOT EXISTS Event (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  task_id INTEGER NULL,
  kind TEXT NOT NULL,
  payload_json TEXT NULL,
  created_at DATETIME NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);

-- triggers to update updated_at
CREATE TRIGGER IF NOT EXISTS trg_task_updated_at
AFTER UPDATE ON Task FOR EACH ROW
BEGIN
  UPDATE Task SET updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE id = OLD.id;
END;

