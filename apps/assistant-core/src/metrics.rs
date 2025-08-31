use std::collections::HashMap;
use std::sync::Mutex;
use once_cell::sync::Lazy;

static API_REQ: Lazy<Mutex<HashMap<String, u64>>> = Lazy::new(|| Mutex::new(HashMap::new()));
static JOBS: Lazy<Mutex<HashMap<(String, String), u64>>> = Lazy::new(|| Mutex::new(HashMap::new()));

pub fn inc_api_request(path: &str) {
    let mut g = API_REQ.lock().unwrap();
    *g.entry(path.to_string()).or_insert(0) += 1;
}

pub fn inc_scheduler_job(job: &str, status: &str) {
    let mut g = JOBS.lock().unwrap();
    *g.entry((job.to_string(), status.to_string())).or_insert(0) += 1;
}

pub fn gather_prometheus(build_version: &str) -> String {
    let mut out = String::new();
    out.push_str(&format!("# TYPE foreman_build_info gauge\nforeman_build_info{{version=\"{}\"}} 1\n", build_version));
    out.push_str("# HELP foreman_api_requests_total API requests total\n# TYPE foreman_api_requests_total counter\n");
    for (k, v) in API_REQ.lock().unwrap().iter() {
        out.push_str(&format!("foreman_api_requests_total{{path=\"{}\"}} {}\n", k.replace('"', "\""), v));
    }
    out.push_str("# HELP foreman_scheduler_jobs_total Scheduler jobs by status\n# TYPE foreman_scheduler_jobs_total counter\n");
    for ((job, status), v) in JOBS.lock().unwrap().iter() {
        out.push_str(&format!("foreman_scheduler_jobs_total{{job=\"{}\",status=\"{}\"}} {}\n", job.replace('"', "\""), status, v));
    }
    out
}

