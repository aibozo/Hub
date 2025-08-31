use once_cell::sync::Lazy;
use prometheus::{Encoder, Registry, TextEncoder, CounterVec, Opts};
use std::sync::Arc;
use tracing_subscriber::{fmt, EnvFilter};

static REGISTRY: Lazy<Registry> = Lazy::new(Registry::new);
static API_REQUESTS: Lazy<CounterVec> = Lazy::new(|| {
    let cv = CounterVec::new(Opts::new("foreman_api_requests_total", "API requests total"), &["path"]).unwrap();
    REGISTRY.register(Box::new(cv.clone())).ok();
    cv
});
static SCHEDULER_JOBS: Lazy<CounterVec> = Lazy::new(|| {
    let cv = CounterVec::new(Opts::new("foreman_scheduler_jobs_total", "Scheduler jobs by status"), &["job","status"]).unwrap();
    REGISTRY.register(Box::new(cv.clone())).ok();
    cv
});

pub fn init() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let fmt = fmt().with_env_filter(filter).with_target(false);
    // Enable JSON logs if FOREMAN_LOG_JSON=1
    if std::env::var("FOREMAN_LOG_JSON").ok().as_deref() == Some("1") {
        fmt.json().init();
    } else {
        fmt.init();
    }
}

pub fn inc_api_request(path: &str) { API_REQUESTS.with_label_values(&[path]).inc(); }
pub fn inc_scheduler_job(job: &str, status: &str) { SCHEDULER_JOBS.with_label_values(&[job, status]).inc(); }

pub fn gather_prometheus() -> String {
    let metric_families = REGISTRY.gather();
    let mut buffer = Vec::new();
    let encoder = TextEncoder::new();
    encoder.encode(&metric_families, &mut buffer).ok();
    String::from_utf8(buffer).unwrap_or_default()
}

