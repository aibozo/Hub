use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use sqlx::{Pool, Row, Sqlite, SqlitePool};
use std::path::Path;

#[derive(Clone)]
pub struct MemoryStore {
    pool: SqlitePool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: i64,
    pub title: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub tags: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskDigest {
    pub task_id: i64,
    pub short: Option<String>,
    pub paragraph: Option<String>,
    pub tokens: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Atom {
    pub id: i64,
    pub task_id: i64,
    pub kind: String,
    pub text: String,
    pub tags: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AtomFull {
    pub id: i64,
    pub task_id: i64,
    pub kind: String,
    pub text: String,
    pub source: String,
    pub source_ref: Option<String>,
    pub importance: i64,
    pub pinned: bool,
    pub tokens_est: i64,
    pub parent_atom_id: Option<i64>,
    pub tags: Option<String>,
    pub hash: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub id: i64,
    pub task_id: Option<i64>,
    pub kind: String,
    pub payload_json: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchHit {
    pub atom_id: i64,
    pub snippet: String,
    pub score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactRow {
    pub id: i64,
    pub task_id: i64,
    pub path: String,
    pub mime: Option<String>,
    pub sha256: Option<String>,
    pub bytes: Option<i64>,
    pub origin_url: Option<String>,
}

impl MemoryStore {
    pub async fn new(db_path: &Path, migrations_dir: &Path) -> Result<Self> {
        if let Some(parent) = db_path.parent() { std::fs::create_dir_all(parent)?; }
        let url = if db_path.is_absolute() {
            format!("sqlite:///{}", db_path.display())
        } else {
            format!("sqlite://{}", db_path.display())
        };
        let pool = SqlitePool::connect(&url).await?;
        // Run migrations from a filesystem path
        let migrator = sqlx::migrate::Migrator::new(migrations_dir).await?;
        migrator.run(&pool).await?;
        Ok(Self { pool })
    }

    pub async fn new_in_memory(migrations_dir: &Path) -> Result<Self> {
        let pool = SqlitePool::connect("sqlite::memory:").await?;
        let migrator = sqlx::migrate::Migrator::new(migrations_dir).await?;
        migrator.run(&pool).await?;
        Ok(Self { pool })
    }

    pub fn pool(&self) -> &Pool<Sqlite> { &self.pool }

    pub async fn append_event(&self, task_id: Option<i64>, kind: &str, payload_json: Option<&JsonValue>) -> Result<i64> {
        let payload = payload_json.map(|v| v.to_string());
        let row = sqlx::query(
            r#"INSERT INTO Event(task_id, kind, payload_json) VALUES (?1, ?2, ?3) RETURNING id"#,
        )
        .bind(task_id)
        .bind(kind)
        .bind(payload)
        .fetch_one(&self.pool)
        .await?;
        Ok(row.get::<i64, _>("id"))
    }

    pub async fn create_task(&self, title: &str, status: &str, tags: Option<&str>) -> Result<Task> {
        let row = sqlx::query(
            r#"INSERT INTO Task(title, status, tags) VALUES (?1, ?2, ?3)
               RETURNING id, title, status, created_at, updated_at, tags"#,
        )
        .bind(title)
        .bind(status)
        .bind(tags)
        .fetch_one(&self.pool)
        .await?;
        Ok(Task {
            id: row.get("id"),
            title: row.get("title"),
            status: row.get("status"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            tags: row.get("tags"),
        })
    }

    pub async fn list_tasks(&self) -> Result<Vec<Task>> {
        let rows = sqlx::query(r#"SELECT id, title, status, created_at, updated_at, tags FROM Task ORDER BY id DESC"#)
            .fetch_all(&self.pool)
            .await?;
        Ok(rows
            .into_iter()
            .map(|r| Task {
                id: r.get("id"),
                title: r.get("title"),
                status: r.get("status"),
                created_at: r.get("created_at"),
                updated_at: r.get("updated_at"),
                tags: r.get("tags"),
            })
            .collect())
    }

    pub async fn upsert_task_digest(&self, task_id: i64, short: Option<&str>, paragraph: Option<&str>, tokens: Option<i64>) -> Result<()> {
        sqlx::query(
            r#"INSERT INTO TaskDigest(task_id, short, paragraph, tokens) VALUES (?1, ?2, ?3, ?4)
               ON CONFLICT(task_id) DO UPDATE SET short=excluded.short, paragraph=excluded.paragraph, tokens=excluded.tokens"#,
        )
        .bind(task_id)
        .bind(short)
        .bind(paragraph)
        .bind(tokens)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn put_atom(&self, task_id: i64, kind: &str, text: &str, tags: Option<&str>) -> Result<i64> {
        let row = sqlx::query(
            r#"INSERT INTO Atom(task_id, kind, text, tags) VALUES (?1, ?2, ?3, ?4) RETURNING id"#,
        )
        .bind(task_id)
        .bind(kind)
        .bind(text)
        .bind(tags)
        .fetch_one(&self.pool)
        .await?;
        Ok(row.get::<i64, _>("id"))
    }

    pub async fn get_atoms_by_task(&self, task_id: i64) -> Result<Vec<Atom>> {
        let rows = sqlx::query(
            r#"SELECT id, task_id, kind, text, tags, created_at FROM Atom WHERE task_id = ?1 ORDER BY id DESC"#,
        )
        .bind(task_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|r| Atom {
                id: r.get("id"),
                task_id: r.get("task_id"),
                kind: r.get("kind"),
                text: r.get("text"),
                tags: r.get("tags"),
                created_at: r.get("created_at"),
            })
            .collect())
    }

    pub async fn get_recent_events(&self, limit: i64) -> Result<Vec<Event>> {
        let rows = sqlx::query(
            r#"SELECT id, task_id, kind, payload_json, created_at FROM Event ORDER BY id DESC LIMIT ?1"#,
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|r| Event {
                id: r.get("id"),
                task_id: r.get("task_id"),
                kind: r.get("kind"),
                payload_json: r.get("payload_json"),
                created_at: r.get("created_at"),
            })
            .collect())
    }

    pub async fn create_artifact(&self, task_id: i64, path: &Path, mime: Option<&str>, sha256: Option<&str>) -> Result<i64> {
        let row = sqlx::query(
            r#"INSERT INTO Artifact(task_id, path, mime, sha256) VALUES (?1, ?2, ?3, ?4) RETURNING id"#,
        )
        .bind(task_id)
        .bind(path.to_string_lossy().to_string())
        .bind(mime)
        .bind(sha256)
        .fetch_one(&self.pool)
        .await?;
        Ok(row.get::<i64, _>("id"))
    }

    pub async fn get_atom_full(&self, id: i64) -> Result<Option<AtomFull>> {
        let row = sqlx::query(
            r#"SELECT id, task_id, kind, text, source, source_ref, importance, pinned, tokens_est, parent_atom_id, tags, hash, created_at
               FROM Atom WHERE id = ?1"#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|r| AtomFull {
            id: r.get("id"),
            task_id: r.get("task_id"),
            kind: r.get("kind"),
            text: r.get("text"),
            source: r.get("source"),
            source_ref: r.get("source_ref"),
            importance: r.get::<i64, _>("importance"),
            pinned: r.get::<i64, _>("pinned") != 0,
            tokens_est: r.get::<i64, _>("tokens_est"),
            parent_atom_id: r.get("parent_atom_id"),
            tags: r.get("tags"),
            hash: r.get("hash"),
            created_at: r.get("created_at"),
        }))
    }

    pub async fn pin_atom(&self, id: i64, pinned: bool) -> Result<()> {
        sqlx::query(r#"UPDATE Atom SET pinned = ?2 WHERE id = ?1"#)
            .bind(id)
            .bind(if pinned { 1 } else { 0 })
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn list_cards(&self, task_id: Option<i64>, limit: i64) -> Result<Vec<AtomFull>> {
        let rows = if let Some(tid) = task_id {
            sqlx::query(
                r#"SELECT id, task_id, kind, text, source, source_ref, importance, pinned, tokens_est, parent_atom_id, tags, hash, created_at
                   FROM Atom
                   WHERE task_id = ?1 AND (pinned = 1 OR importance >= 2)
                   ORDER BY pinned DESC, importance DESC, created_at DESC
                   LIMIT ?2"#,
            )
            .bind(tid)
            .bind(limit)
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query(
                r#"SELECT id, task_id, kind, text, source, source_ref, importance, pinned, tokens_est, parent_atom_id, tags, hash, created_at
                   FROM Atom
                   WHERE (pinned = 1 OR importance >= 2)
                   ORDER BY pinned DESC, importance DESC, created_at DESC
                   LIMIT ?1"#,
            )
            .bind(limit)
            .fetch_all(&self.pool)
            .await?
        };

        Ok(rows
            .into_iter()
            .map(|r| AtomFull {
                id: r.get("id"),
                task_id: r.get("task_id"),
                kind: r.get("kind"),
                text: r.get("text"),
                source: r.get("source"),
                source_ref: r.get("source_ref"),
                importance: r.get::<i64, _>("importance"),
                pinned: r.get::<i64, _>("pinned") != 0,
                tokens_est: r.get::<i64, _>("tokens_est"),
                parent_atom_id: r.get("parent_atom_id"),
                tags: r.get("tags"),
                hash: r.get("hash"),
                created_at: r.get("created_at"),
            })
            .collect())
    }

    pub async fn search_atoms(&self, q: &str, task_id: Option<i64>, k: i64) -> Result<Vec<SearchHit>> {
        let sql_any = r#"
            SELECT a.id as atom_id,
                   snippet(atom_fts, 0, '[', ']', ' … ', 8) as snippet,
                   bm25(atom_fts) as score
            FROM atom_fts
            JOIN Atom a ON a.id = atom_fts.rowid
            WHERE atom_fts MATCH ?1
            ORDER BY a.pinned DESC, a.importance DESC, score ASC, a.created_at DESC
            LIMIT ?2
        "#;
        let sql_task = r#"
            SELECT a.id as atom_id,
                   snippet(atom_fts, 0, '[', ']', ' … ', 8) as snippet,
                   bm25(atom_fts) as score
            FROM atom_fts
            JOIN Atom a ON a.id = atom_fts.rowid
            WHERE atom_fts MATCH ?1 AND a.task_id = ?2
            ORDER BY a.pinned DESC, a.importance DESC, score ASC, a.created_at DESC
            LIMIT ?3
        "#;
        let rows = if let Some(tid) = task_id {
            sqlx::query(sql_task)
                .bind(q)
                .bind(tid)
                .bind(k)
                .fetch_all(&self.pool)
                .await?
        } else {
            sqlx::query(sql_any)
                .bind(q)
                .bind(k)
                .fetch_all(&self.pool)
                .await?
        };
        Ok(rows
            .into_iter()
            .map(|r| SearchHit { atom_id: r.get("atom_id"), snippet: r.get("snippet"), score: r.get::<f64, _>("score") })
            .collect())
    }

    pub async fn get_artifact(&self, id: i64) -> Result<Option<ArtifactRow>> {
        let row = sqlx::query(
            r#"SELECT id, task_id, path, mime, sha256, bytes, origin_url FROM Artifact WHERE id = ?1"#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|r| ArtifactRow {
            id: r.get("id"),
            task_id: r.get("task_id"),
            path: r.get::<String, _>("path"),
            mime: r.get("mime"),
            sha256: r.get("sha256"),
            bytes: r.get("bytes"),
            origin_url: r.get("origin_url"),
        }))
    }
}
