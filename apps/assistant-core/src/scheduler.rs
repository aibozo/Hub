use chrono::{DateTime, Local, NaiveTime};
use serde::Serialize;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::memory::Memory;
use crate::tools::ToolsManager;
use serde_json::json;

#[derive(Clone, Debug)]
pub struct SchedulerConfig {
    pub timezone: String,
    pub jobs: HashMap<String, String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct JobState {
    pub name: String,
    pub schedule: String,
    pub next_run: Option<String>,
    pub last_run: Option<String>,
    pub last_status: Option<String>,
}

#[derive(Clone)]
pub struct Scheduler {
    cfg: SchedulerConfig,
    state: Arc<Mutex<HashMap<String, JobState>>>,
    home: PathBuf,
    memory: Option<Memory>,
    tools: ToolsManager,
}

impl SchedulerConfig {
    pub fn load_from_file(path: &str) -> SchedulerConfig { super::scheduler::Scheduler::load_from_file(path) }
}

impl Scheduler {
    pub fn new(cfg: SchedulerConfig, home: PathBuf, memory: Option<Memory>, tools: ToolsManager) -> Self {
        let now = Local::now();
        let state = cfg.jobs.iter().map(|(k, v)| {
            let next = Self::compute_next(v, now).map(|d| d.to_rfc3339());
            (k.clone(), JobState { name: k.clone(), schedule: v.clone(), next_run: next, last_run: None, last_status: None })
        }).collect();
        Self { cfg, state: Arc::new(Mutex::new(state)), home, memory, tools }
    }

    pub fn load_from_file(path: &str) -> SchedulerConfig {
        let text = std::fs::read_to_string(path).unwrap_or_else(|_| "timezone = \"America/Indiana/Indianapolis\"\n[jobs]\narxiv=\"07:30\"\nnews=\"08:00\"\n".to_string());
        let v: toml::Value = toml::from_str(&text).unwrap_or(toml::Value::Table(Default::default()));
        let tz = v.get("timezone").and_then(|x| x.as_str()).unwrap_or("America/Indiana/Indianapolis").to_string();
        let mut jobs = HashMap::new();
        if let Some(tbl) = v.get("jobs").and_then(|x| x.as_table()) {
            for (k, val) in tbl.iter() {
                if let Some(s) = val.as_str() { jobs.insert(k.clone(), s.to_string()); }
            }
        }
        if jobs.is_empty() {
            jobs.insert("arxiv".into(), "07:30".into());
            jobs.insert("news".into(), "08:00".into());
        }
        SchedulerConfig { timezone: tz, jobs }
    }

    pub fn start(self) {
        let this = self.clone();
        tokio::spawn(async move { this.run_loop().await; });
    }

    async fn run_loop(self) {
        loop {
            let now = Local::now();
            let mut min_dt: Option<DateTime<Local>> = None;
            let mut due: Vec<String> = vec![];
            {
                let st = self.state.lock().await.clone();
                for (name, js) in st.iter() {
                    if let Some(next) = Self::compute_next(&js.schedule, now) {
                        if min_dt.map(|d| next < d).unwrap_or(true) { min_dt = Some(next); }
                        if next <= now { due.push(name.clone()); }
                    }
                }
            }

            for job in due {
                let res = self.run_job(&job).await;
                let mut st = self.state.lock().await;
                if let Some(js) = st.get_mut(&job) {
                    js.last_run = Some(Local::now().to_rfc3339());
                    let status = if res.is_ok() { "ok" } else { "error" };
                    js.last_status = Some(status.to_string());
                    crate::metrics::inc_scheduler_job(&js.name, status);
                    js.next_run = Self::compute_next(&js.schedule, Local::now()).map(|d| d.to_rfc3339());
                }
            }

            let sleep_ms = min_dt.map(|d| (d - Local::now()).num_milliseconds().max(1000) as u64).unwrap_or(60_000);
            tokio::time::sleep(std::time::Duration::from_millis(sleep_ms)).await;
            let mut st = self.state.lock().await;
            for js in st.values_mut() {
                js.next_run = Self::compute_next(&js.schedule, Local::now()).map(|d| d.to_rfc3339());
            }
        }
    }

    fn compute_next(hhmm: &str, now: DateTime<Local>) -> Option<DateTime<Local>> {
        let parts: Vec<&str> = hhmm.split(':').collect();
        if parts.len() != 2 { return None; }
        let h: u32 = parts[0].parse().ok()?;
        let m: u32 = parts[1].parse().ok()?;
        let today = now.date_naive();
        let time = NaiveTime::from_hms_opt(h, m, 0)?;
        let dt_today = today.and_time(time);
        let mut candidate = dt_today.and_local_timezone(Local).single()?;
        if candidate <= now {
            let next_day = today.succ_opt()?;
            let next_dt = next_day.and_time(time);
            candidate = next_dt.and_local_timezone(Local).single()?;
        }
        Some(candidate)
    }

    pub async fn snapshot(&self) -> Vec<JobState> {
        self.state.lock().await.values().cloned().collect()
    }

    async fn run_job(&self, name: &str) -> anyhow::Result<()> {
        match name {
            "arxiv" => self.run_brief_job("arxiv").await,
            "news" => self.run_brief_job("news").await,
            _ => Ok(())
        }
    }

    async fn ensure_task(&self) -> anyhow::Result<i64> {
        if let Some(mem) = self.memory.as_ref() {
            let t = mem.store.create_task("Daily Briefs", "open", Some("briefs")).await?;
            return Ok(t.id);
        }
        anyhow::bail!("memory not available")
    }

    async fn run_brief_job(&self, kind: &str) -> anyhow::Result<()> {
        let task_id = self.ensure_task().await?;
        let date = Local::now().format("%Y-%m-%d").to_string();
        let fname = format!("{}-{}.md", date, kind);
        let briefs_dir = self.home.join("briefs");
        tokio::fs::create_dir_all(&briefs_dir).await.ok();
        let path = briefs_dir.join(fname);
        let content = match kind {
            "arxiv" => {
                let month = Local::now().format("%Y-%m").to_string();
                let res = self.tools.invoke("arxiv", "top", json!({"month": month, "n": 5})).await.unwrap_or_else(|_| json!({"items": []}));
                let items = res.get("items").and_then(|v| v.as_array()).cloned().unwrap_or_default();
                let mut md = format!("# arXiv Top Papers ({})\n\n", date);
                for it in items {
                    let title = it.get("title").and_then(|v| v.as_str()).unwrap_or("");
                    let id = it.get("id").and_then(|v| v.as_str()).unwrap_or("");
                    md.push_str(&format!("- [{}] {}\n", id, title));
                }
                md
            }
            "news" => {
                let res = self.tools.invoke("news", "daily_brief", json!({"categories": ["world","tech"]})).await.unwrap_or_else(|_| json!({"markdown": "# News Brief\n\n(no data)"}));
                res.get("markdown").and_then(|v| v.as_str()).unwrap_or("# News Brief\n\n(no data)").to_string()
            }
            _ => format!("# {} Brief\n\nThis is a placeholder brief generated by the scheduler.\n\n- Date: {}\n- Kind: {}\n", kind, date, kind)
        };
        tokio::fs::write(&path, content.as_bytes()).await?;
        if let Some(mem) = self.memory.as_ref() {
            let _ = mem.store.append_event(
                Some(task_id),
                &format!("scheduler:{}:run", kind),
                Some(&json!({"artifact_path": path})),
            ).await;
            let _ = mem.store.put_atom(task_id, "brief", &format!("{} brief created: artifact://{}", kind, path.to_string_lossy()), Some("brief")).await;
            let _ = mem.store.create_artifact(task_id, &path, Some("text/markdown"), None).await;
        }
        Ok(())
    }

    pub async fn run_now(&self, job: &str) -> anyhow::Result<()> {
        self.run_job(job).await
    }
}
