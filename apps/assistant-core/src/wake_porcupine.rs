#![cfg(feature = "wake-porcupine")]

use anyhow::Context as _;

pub struct PorcupineDetector {
    inner: pv_porcupine::Porcupine,
    pub frame_length: usize,
    pub sample_rate: u32,
}

pub struct PorcupineOpts {
    pub access_key_env: String,
    pub keyword_path: String,
    pub model_path: Option<String>,
    pub sensitivity: f32,
}

impl PorcupineDetector {
    pub fn new(opts: PorcupineOpts) -> anyhow::Result<Self> {
        let access_key = std::env::var(&opts.access_key_env)
            .or_else(|_| std::env::var("PICOVOICE_ACCESS_KEY"))
            .context("PICOVOICE access key not set in env")?;

        let mut builder = pv_porcupine::PorcupineBuilder::new_with_keyword_paths(&[opts.keyword_path.clone()])
            .map_err(|e| anyhow::anyhow!(format!("porcupine builder: {:?}", e)))?;
        builder = builder.access_key(access_key);
        if let Some(mp) = opts.model_path.as_ref() { builder = builder.model_path(mp.clone()); }
        builder = builder.sensitivities(&[opts.sensitivity]).map_err(|e| anyhow::anyhow!(format!("porcupine sensitivities: {:?}", e)))?;
        let inner = builder.init().map_err(|e| anyhow::anyhow!(format!("porcupine init: {:?}", e)))?;
        let frame_length = inner.frame_length() as usize;
        let sample_rate = inner.sample_rate() as u32;
        Ok(Self { inner, frame_length, sample_rate })
    }

    pub fn process(&mut self, frame: &[i16]) -> anyhow::Result<bool> {
        let res = self.inner.process(frame).map_err(|e| anyhow::anyhow!(format!("porcupine process: {:?}", e)))?;
        Ok(res >= 0)
    }
}

