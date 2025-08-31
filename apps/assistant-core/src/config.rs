use serde::{Deserialize, Serialize};
use std::{env, fs, path::PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceConfig {
    pub wake_phrase: Option<String>,
    #[serde(default)]
    pub wake_enabled: Option<bool>,
    #[serde(default)]
    pub vad_sensitivity: Option<f32>,
    #[serde(default)]
    pub min_speech_ms: Option<u32>,
    #[serde(default)]
    pub refractory_ms: Option<u32>,
    #[serde(default)]
    pub stt: serde_json::Value,
    #[serde(default)]
    pub tts: serde_json::Value,
    #[serde(default)]
    pub porcupine: Option<PorcupineConfig>,
    #[serde(default)]
    pub realtime_endpoint: Option<String>,
    #[serde(default)]
    pub realtime_model: Option<String>,
    #[serde(default)]
    pub realtime_voice: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PorcupineConfig {
    /// Path to keyword .ppn file (preferred). If not set, keyword_dir will be scanned for first .ppn.
    pub keyword_path: Option<String>,
    /// Directory to scan for .ppn if keyword_path is not provided.
    pub keyword_dir: Option<String>,
    /// Optional path to porcupine model .pv file
    pub model_path: Option<String>,
    /// 0.0..1.0 sensitivity (default 0.5)
    pub sensitivity: Option<f32>,
    /// Env var name holding the access key (default PICOVOICE_ACCESS_KEY)
    pub access_key_env: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulesConfig {
    pub arxiv_brief: Option<String>,
    pub news_brief: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpConfig {
    #[serde(default)]
    pub servers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForemanConfig {
    pub home: Option<String>,
    pub profile: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    pub foreman: Option<ForemanConfig>,
    pub voice: Option<VoiceConfig>,
    pub schedules: Option<SchedulesConfig>,
    pub mcp: Option<McpConfig>,
}

impl Config {
    pub fn load() -> anyhow::Result<(Self, PathBuf)> {
        let cfg_path = env::var("FOREMAN_CONFIG")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("config/foreman.toml"));
        let text = fs::read_to_string(&cfg_path)?;
        let mut cfg: Config = toml::from_str(&text)?;

        // Env overrides (minimal): FOREMAN_HOME, FOREMAN_PROFILE
        if let Some(home) = env::var("FOREMAN_HOME").ok() {
            cfg.foreman.get_or_insert(ForemanConfig { home: None, profile: None }).home = Some(home);
        }
        if let Some(profile) = env::var("FOREMAN_PROFILE").ok() {
            cfg.foreman.get_or_insert(ForemanConfig { home: None, profile: None }).profile = Some(profile);
        }

        Ok((cfg, cfg_path))
    }

    pub fn home_dir(&self) -> PathBuf {
        self.foreman
            .as_ref()
            .and_then(|f| f.home.as_ref())
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("./storage"))
    }
}
