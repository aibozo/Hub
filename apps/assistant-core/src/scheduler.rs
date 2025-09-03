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
                // Run research pipeline (bounded)
                let budgets = crate::research::types::ResearchBudgets { max_papers: 12, max_title_chars: 160, max_summary_chars: 800 };
                let params = crate::research::types::ResearchTaskParams { query: "recent arXiv cs.*".into(), categories: vec![], window_days: 3, limit: 25, budgets };
                let mut bundle = crate::research::pipeline::run_pipeline(&self.tools, &params).await.unwrap_or_else(|_| crate::research::types::ReportBundle { kind: "research_report/v1".into(), topic: "arxiv".into(), generated_at: chrono::Utc::now().to_rfc3339(), sources: vec![] });
                // Optional multiagent selection (env-gated)
                if std::env::var("RESEARCH_MULTIAGENT").ok().as_deref() == Some("1") {
                    let top_k: usize = std::env::var("RESEARCH_TOP_K").ok().and_then(|s| s.parse().ok()).unwrap_or(6);
                    let shards: usize = std::env::var("RESEARCH_SHARDS").ok().and_then(|s| s.parse().ok()).unwrap_or(2);
                    let per_tokens: usize = std::env::var("RESEARCH_WORKER_TOKENS").ok().and_then(|s| s.parse().ok()).unwrap_or(512);
                    let opts = crate::research::agents::orchestrator::OrchestratorOptions { shards, per_worker_tokens: per_tokens, top_k };
                    let agg = crate::research::agents::orchestrator::orchestrate(&bundle, &opts);
                    // Filter sources to selected ids, preserving order of selection
                    let selected: std::collections::HashSet<String> = agg.selected_ids.iter().cloned().collect();
                    let mut filtered: Vec<crate::research::types::PaperMini> = vec![];
                    for id in agg.selected_ids.iter() {
                        if let Some(p) = bundle.sources.iter().find(|p| &p.id == id) { filtered.push(p.clone()); }
                    }
                    if !filtered.is_empty() { bundle.sources = filtered; }
                    // Write highlights to a sidecar file
                    let hl_path = briefs_dir.join(format!("{}-{}-highlights.txt", date, kind));
                    let _ = tokio::fs::write(&hl_path, agg.highlights.join("\n")).await;
                }
                // Sidecar JSON
                let json_path = briefs_dir.join(format!("{}-{}.json", date, kind));
                let _ = tokio::fs::write(&json_path, serde_json::to_vec_pretty(&bundle).unwrap_or_else(|_| b"{}".to_vec())).await;
                // Deterministic markdown synthesis (no model here)
                let mut md = format!("# arXiv Brief — {}\n\n", date);
                for (i, p) in bundle.sources.iter().enumerate() {
                    md.push_str(&format!("{}. {}\n", i+1, p.title));
                    if !p.authors.is_empty() { md.push_str(&format!("   - Authors: {}\n", p.authors.join(", "))); }
                    if let Some(u) = &p.html_url { md.push_str(&format!("   - Abs: {}\n", u)); }
                    if let Some(u) = &p.pdf_url { md.push_str(&format!("   - PDF: {}\n", u)); }
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
