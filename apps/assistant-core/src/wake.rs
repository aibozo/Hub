use parking_lot::RwLock;
use std::sync::Arc;

#[cfg(feature = "realtime-audio")]
use std::collections::VecDeque;

#[cfg(feature = "realtime-audio")]
use tokio::sync::mpsc;
#[cfg(feature = "realtime-audio")]
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Clone)]
pub struct WakeOptions {
    pub phrase: String,
    pub enabled: bool,
    pub vad_sensitivity: f32, // 0.0..1.0 higher is more sensitive
    pub min_speech_ms: u32,
    pub refractory_ms: u64,
}

#[derive(Clone)]
pub struct WakeSentinel {
    inner: Arc<RwLock<Inner>>, 
}

struct Inner {
    opts: WakeOptions,
    active: bool,
    stop: Option<tokio::sync::oneshot::Sender<()>>,
}

impl Default for WakeOptions {
    fn default() -> Self {
        Self { phrase: "hey vim".into(), enabled: false, vad_sensitivity: 0.5, min_speech_ms: 400, refractory_ms: 3000 }
    }
}

impl WakeSentinel {
    pub fn new(opts: WakeOptions) -> Self { Self { inner: Arc::new(RwLock::new(Inner { opts, active: false, stop: None })) } }

    pub fn status(&self) -> (bool, WakeOptions) {
        let g = self.inner.read();
        (g.active, g.opts.clone())
    }

    pub fn set_enabled(&self, enabled: bool) { self.inner.write().opts.enabled = enabled; }

    #[cfg(feature = "realtime-audio")]
    pub async fn start_task(&self, realtime: crate::realtime::RealtimeManager) {
        let mut g = self.inner.write();
        if g.active { return; }
        let (tx, mut rx_stop) = tokio::sync::oneshot::channel::<()>();
        g.stop = Some(tx);
        g.active = true;
        let opts = g.opts.clone();
        drop(g);

        tokio::spawn(async move {
            if !opts.enabled { return; }
            // Start capture in a dedicated thread to avoid Send bounds; deliver frames via tokio mpsc
            let (tx_frames, mut rx_frames) = mpsc::channel::<Vec<i16>>(16);
            let stop_flag = Arc::new(AtomicBool::new(false));
            let stop_for_thread = stop_flag.clone();
            std::thread::spawn(move || {
                match crate::realtime_audio::start_capture(16000, 30, tx_frames) {
                    Ok(_cap) => {
                        // Keep stream alive until stop flag is set
                        while !stop_for_thread.load(Ordering::SeqCst) { std::thread::sleep(std::time::Duration::from_millis(100)); }
                        // _cap dropped here; stream stops
                    }
                    Err(e) => { eprintln!("[wake] capture error: {}", e); }
                }
            });
            let mut last_trigger = std::time::Instant::now() - std::time::Duration::from_millis(opts.refractory_ms);
            let mut ring: VecDeque<i16> = VecDeque::with_capacity(16000 * 3);
            let mut in_speech = false;
            let mut speech_len_ms: u32 = 0;
            let mut phrase_buf: Vec<i16> = vec![];
            loop {
                tokio::select! {
                    _ = &mut rx_stop => { stop_flag.store(true, Ordering::SeqCst); break; }
                    Some(frame) = rx_frames.recv() => {
                        // Maintain ring buffer (3s)
                        for s in &frame { if ring.len()>=16000*3 { ring.pop_front(); } ring.push_back(*s); }
                        // VAD energy
                        let energy: f32 = frame.iter().map(|s| s.abs() as f32 / 32768.0).sum::<f32>() / (frame.len().max(1) as f32);
                        let thr: f32 = 0.010; // fixed threshold
                        if energy > thr {
                            in_speech = true; speech_len_ms += 30; phrase_buf.extend_from_slice(&frame);
                        } else {
                            if in_speech {
                                // Speech ended
                                in_speech = false;
                                if speech_len_ms >= opts.min_speech_ms {
                                    // Transcribe and match wake phrase
                                    let text = transcribe(&phrase_buf, 16000).await;
                                    if matches_wake(&text, &opts.phrase) && last_trigger.elapsed().as_millis() as u64 >= opts.refractory_ms {
                                        let _ = realtime.start(crate::realtime::RealtimeOptions { model: Some("gpt-realtime".into()), voice: Some("alloy".into()), audio: Some(crate::realtime::RealtimeAudioOpts { in_sr: Some(16000), out_format: Some("pcm16".into()) }), instructions: None, endpoint: None, transport: None }).await;
                                        last_trigger = std::time::Instant::now();
                                    }
                                }
                                speech_len_ms = 0; phrase_buf.clear();
                            }
                        }
                    }
                }
            }
        });
    }

    pub async fn stop_task(&self) {
        if let Some(tx) = self.inner.write().stop.take() { let _ = tx.send(()); }
        self.inner.write().active = false;
    }
}

async fn transcribe(_pcm: &[i16], _sr: u32) -> String {
    if std::env::var("OPENAI_API_KEY").is_ok() {
        if let Ok(t) = crate::stt::transcribe_openai_pcm16(_pcm, _sr).await { return t; }
    }
    String::new()
}

fn normalize(s: &str) -> String {
    s.to_lowercase().chars().filter(|c| c.is_ascii_alphanumeric() || c.is_whitespace()).collect::<String>().split_whitespace().collect::<Vec<_>>().join(" ")
}

fn edit_distance(a: &str, b: &str) -> usize {
    let a = a.as_bytes(); let b = b.as_bytes();
    let mut dp = vec![0..=b.len()].into_iter().flatten().collect::<Vec<usize>>();
    let mut prev;
    for (i, &ac) in a.iter().enumerate() { prev = dp[0]; dp[0] = i+1; for (j, &bc) in b.iter().enumerate() { let tmp = dp[j+1]; dp[j+1] = ((dp[j+1]+1).min(dp[j]+1)).min(prev + if ac==bc {0} else {1}); prev = tmp; } }
    *dp.last().unwrap_or(&0)
}

fn matches_wake(text: &str, phrase: &str) -> bool {
    let t = normalize(text);
    let p = normalize(phrase);
    if t.contains(&p) { return true; }
    // tolerate small edit distance on compact form
    let tc: String = t.chars().filter(|c| !c.is_whitespace()).collect();
    let pc: String = p.chars().filter(|c| !c.is_whitespace()).collect();
    edit_distance(&tc, &pc) <= 2
}

#[cfg(test)]
mod tests {
    use super::matches_wake;
    #[test]
    fn test_matches() {
        assert!(matches_wake("hey vim", "hey vim"));
        assert!(matches_wake("heyvim", "hey vim"));
        assert!(matches_wake("hey, vim!", "hey vim"));
        assert!(matches_wake("hay vim", "hey vim"));
        assert!(!matches_wake("hello there", "hey vim"));
    }
}
