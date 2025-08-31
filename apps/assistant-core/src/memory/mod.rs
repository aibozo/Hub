pub mod context_pack;

use foreman_memory as fm;
use std::path::PathBuf;

#[derive(Clone)]
pub struct Memory {
    pub store: fm::MemoryStore,
}

impl Memory {
    pub async fn init(db_path: PathBuf, migrations_dir: PathBuf) -> anyhow::Result<Self> {
        let store = fm::MemoryStore::new(&db_path, &migrations_dir).await?;
        let this = Self { store };
        // Enable sane SQLite pragmas for durability/perf
        let _ = sqlx::query("PRAGMA journal_mode=WAL").execute(this.store.pool()).await;
        let _ = sqlx::query("PRAGMA synchronous=NORMAL").execute(this.store.pool()).await;
        Ok(this)
    }

    pub async fn init_in_memory(migrations_dir: PathBuf) -> anyhow::Result<Self> {
        let store = fm::MemoryStore::new_in_memory(&migrations_dir).await?;
        Ok(Self { store })
    }
}
