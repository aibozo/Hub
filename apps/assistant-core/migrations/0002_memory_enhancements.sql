-- 0002: Memory enhancements (atoms, events, artifacts) + FTS5 for Atom.text

-- NOTE: Atom.kind already exists from 0001
ALTER TABLE Atom ADD COLUMN source TEXT NOT NULL DEFAULT 'user';
ALTER TABLE Atom ADD COLUMN source_ref TEXT NULL;
ALTER TABLE Atom ADD COLUMN importance INTEGER NOT NULL DEFAULT 0;
ALTER TABLE Atom ADD COLUMN pinned INTEGER NOT NULL DEFAULT 0; -- 0/1
ALTER TABLE Atom ADD COLUMN tokens_est INTEGER NOT NULL DEFAULT 0;
ALTER TABLE Atom ADD COLUMN parent_atom_id INTEGER NULL REFERENCES Atom(id) ON DELETE SET NULL;
ALTER TABLE Atom ADD COLUMN hash TEXT NULL;

CREATE INDEX IF NOT EXISTS idx_atom_task ON Atom(task_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_atom_pinned ON Atom(pinned, importance DESC, created_at DESC);
CREATE UNIQUE INDEX IF NOT EXISTS idx_atom_hash ON Atom(hash);

-- 2) Artifacts & Events provenance
ALTER TABLE Artifact ADD COLUMN bytes INTEGER NULL;
ALTER TABLE Artifact ADD COLUMN origin_url TEXT NULL;
-- mime and sha256 already exist in 0001

ALTER TABLE Event ADD COLUMN actor TEXT NOT NULL DEFAULT 'agent';
ALTER TABLE Event ADD COLUMN tool TEXT NULL;
ALTER TABLE Event ADD COLUMN approval_id TEXT NULL;
ALTER TABLE Event ADD COLUMN plan_id TEXT NULL;

-- 3) FTS5 external-content index for Atom.text
-- Some SQLite builds need "load_extension" enabled for fts5; sqlx/sqlite includes FTS5.
CREATE VIRTUAL TABLE IF NOT EXISTS atom_fts
USING fts5(text, content='Atom', content_rowid='id', tokenize='porter');

-- Initial backfill from existing Atom rows
INSERT INTO atom_fts(rowid, text)
SELECT id, text FROM Atom;

-- Sync triggers
CREATE TRIGGER IF NOT EXISTS atom_ai AFTER INSERT ON Atom BEGIN
  INSERT INTO atom_fts(rowid, text) VALUES (new.id, new.text);
END;
CREATE TRIGGER IF NOT EXISTS atom_ad AFTER DELETE ON Atom BEGIN
  INSERT INTO atom_fts(atom_fts, rowid, text) VALUES('delete', old.id, old.text);
END;
CREATE TRIGGER IF NOT EXISTS atom_au AFTER UPDATE OF text ON Atom BEGIN
  INSERT INTO atom_fts(atom_fts, rowid, text) VALUES('delete', old.id, old.text);
  INSERT INTO atom_fts(rowid, text) VALUES (new.id, new.text);
END;
