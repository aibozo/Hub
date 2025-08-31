use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HardwareInfo {
    pub cpu_model: Option<String>,
    pub gpu_model: Option<String>,
    pub ram_gb: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OsInfo {
    pub name: String,
    pub version: Option<String>,
    pub kernel: Option<String>,
    pub arch: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RuntimesInfo {
    pub python: Option<String>,
    pub node: Option<String>,
    pub rustc: Option<String>,
    pub cargo: Option<String>,
    pub java: Option<String>,
    pub cuda: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DevEnvInfo {
    pub editors: Vec<String>,
    pub vcs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NetworkInfo {
    pub hostname: Option<String>,
    pub interfaces: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SystemMap {
    pub scanned_at: DateTime<Utc>,
    pub hardware: HardwareInfo,
    pub os: OsInfo,
    pub runtimes: RuntimesInfo,
    pub package_managers: Vec<String>,
    pub apps: Vec<String>,
    pub dev_env: DevEnvInfo,
    pub network: NetworkInfo,
}

