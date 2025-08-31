#[cfg(feature = "realtime-audio")]
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

#[cfg(feature = "realtime-audio")]
use ringbuf::{Producer, RingBuffer};

#[cfg(feature = "realtime-audio")]
use std::sync::{Arc, Mutex};

#[cfg(feature = "realtime-audio")]
pub fn decode_ulaw_to_pcm(ulaw: &[u8]) -> Vec<i16> {
    // ITU-T G.711 µ-law decode
    fn ulaw_to_linear(sample: u8) -> i16 {
        const BIAS: i16 = 0x84;
        let sample = !sample;
        let sign = (sample & 0x80) != 0;
        let exponent = ((sample >> 4) & 0x07) as i16;
        let mantissa = (sample & 0x0F) as i16;
        let mut magnitude = ((mantissa << 4) + 0x08) << exponent;
        magnitude += BIAS;
        let linear = if sign { BIAS - magnitude } else { magnitude - BIAS };
        linear
    }
    ulaw.iter().map(|&b| ulaw_to_linear(b)).collect()
}

#[cfg(feature = "realtime-audio")]
pub fn resample_linear_i16(input: &[i16], in_sr: u32, out_sr: u32) -> Vec<i16> {
    if in_sr == out_sr || input.is_empty() { return input.to_vec(); }
    let ratio = out_sr as f32 / in_sr as f32;
    let out_len = ((input.len() as f32) * ratio).round().max(1.0) as usize;
    let mut out = Vec::with_capacity(out_len);
    for i in 0..out_len {
        let src_pos = (i as f32) / ratio;
        let idx0 = src_pos.floor() as usize;
        let idx1 = (idx0 + 1).min(input.len().saturating_sub(1));
        let t = src_pos - (idx0 as f32);
        let s0 = input[idx0] as f32;
        let s1 = input[idx1] as f32;
        let v = s0 + (s1 - s0) * t;
        out.push(v.clamp(i16::MIN as f32, i16::MAX as f32) as i16);
    }
    out
}

#[cfg(feature = "realtime-audio")]
pub struct AudioPlayback {
    _stream: cpal::Stream,
    // Shared ring buffer for PCM i16 samples
    ring: Arc<Mutex<Producer<i16>>>,
    device_sr: u32,
}

// Provide a stub type when audio feature is disabled so other modules
// can still reference `realtime_audio::AudioPlayback` in signatures.
#[cfg(not(feature = "realtime-audio"))]
pub struct AudioPlayback;

#[cfg(feature = "realtime-audio")]
impl AudioPlayback {
    pub fn new(desired_sr: u32) -> anyhow::Result<Self> {
        let host = cpal::default_host();
        let device = host.default_output_device().ok_or_else(|| anyhow::anyhow!("no output device"))?;
        let mut chosen: Option<cpal::SupportedStreamConfig> = None;
        for range in device.supported_output_configs().map_err(|e| anyhow::anyhow!("output configs: {}", e))? {
            if range.sample_format() == cpal::SampleFormat::I16 {
                if range.min_sample_rate().0 <= desired_sr && range.max_sample_rate().0 >= desired_sr {
                    chosen = Some(range.with_sample_rate(cpal::SampleRate(desired_sr)));
                    break;
                } else if chosen.is_none() {
                    chosen = Some(range.with_sample_rate(range.min_sample_rate()));
                }
            }
        }
        let config = chosen.expect("no I16 output config available");

        let device_sr = config.sample_rate().0;
        let cfg: cpal::StreamConfig = config.into();
        // Create a ring buffer for ~60s of audio to tolerate bursty delivery
        let capacity = (device_sr as usize) * 60; // 60.0s mono
        let rb = RingBuffer::<i16>::new(capacity);
        let (prod, mut cons) = rb.split();
        let ring = Arc::new(Mutex::new(prod));
        let ring_cb = ring.clone();

        let stream = device.build_output_stream(
            &cfg,
            move |data: &mut [i16], _: &cpal::OutputCallbackInfo| {
                for s in data.iter_mut() {
                    if let Some(sample) = cons.pop() { *s = sample; } else { *s = 0; }
                }
            },
            move |err| {
                eprintln!("[audio] output error: {}", err);
                let _ = ring_cb; // keep alive
            },
            None,
        )?;
        stream.play()?;
        Ok(Self { _stream: stream, ring, device_sr })
    }

    pub fn push_pcm(&self, pcm: &[i16], src_sr: u32) {
        let samples = if src_sr == self.device_sr { pcm.to_vec() } else { resample_linear_i16(pcm, src_sr, self.device_sr) };
        if let Ok(mut prod) = self.ring.lock() {
            for s in samples { let _ = prod.push(s); }
        }
    }

    pub fn device_sr(&self) -> u32 { self.device_sr }
}

#[cfg(feature = "realtime-audio")]
pub struct AudioCapture {
    _stream: cpal::Stream,
    // Send captured frames (PCM i16 chunk) to a tokio channel
}

#[cfg(feature = "realtime-audio")]
pub fn start_capture(desired_sr: u32, chunk_ms: u32, tx: tokio::sync::mpsc::Sender<Vec<i16>>) -> anyhow::Result<AudioCapture> {
    let host = cpal::default_host();
    let device = host.default_input_device().ok_or_else(|| anyhow::anyhow!("no input device"))?;
    let mut chosen: Option<cpal::SupportedStreamConfig> = None;
    for range in device.supported_input_configs().map_err(|e| anyhow::anyhow!("input configs: {}", e))? {
        if range.sample_format() == cpal::SampleFormat::I16 {
            if range.min_sample_rate().0 <= desired_sr && range.max_sample_rate().0 >= desired_sr {
                chosen = Some(range.with_sample_rate(cpal::SampleRate(desired_sr)));
                break;
            } else if chosen.is_none() {
                chosen = Some(range.with_sample_rate(range.min_sample_rate()));
            }
        }
    }
    let config = chosen.expect("no I16 input config available");
    let cfg: cpal::StreamConfig = config.into();
    let frames_per_chunk = (desired_sr as usize) * (chunk_ms as usize) / 1000;
    let mut buffer: Vec<i16> = Vec::with_capacity(frames_per_chunk);
    let stream = device.build_input_stream(
        &cfg,
        move |data: &[i16], _info: &cpal::InputCallbackInfo| {
            for &s in data {
                buffer.push(s);
                if buffer.len() >= frames_per_chunk {
                    let mut out = Vec::with_capacity(buffer.len());
                    out.extend_from_slice(&buffer);
                    buffer.clear();
                    let _ = tx.try_send(out);
                }
            }
        },
        move |err| {
            eprintln!("[audio] input error: {}", err);
        },
        None,
    )?;
    stream.play()?;
    Ok(AudioCapture { _stream: stream })
}

// --- Diagnostics helpers (devices, VAD capture, beep) ---

#[cfg(feature = "realtime-audio")]
pub fn devices_info_json() -> serde_json::Value {
    let host = cpal::default_host();
    let host_name = format!("{:?}", host.id());
    let input_name = host.default_input_device().and_then(|d| d.name().ok()).unwrap_or_else(|| "(none)".into());
    let output_name = host.default_output_device().and_then(|d| d.name().ok()).unwrap_or_else(|| "(none)".into());
    // Try a few supported rates for I16
    let mut input_rates: Vec<u32> = vec![];
    if let Some(dev) = host.default_input_device() {
        if let Ok(cfgs) = dev.supported_input_configs() {
            for range in cfgs {
                if range.sample_format() == cpal::SampleFormat::I16 {
                    input_rates.push(range.min_sample_rate().0);
                    input_rates.push(range.max_sample_rate().0);
                    break;
                }
            }
        }
    }
    let mut output_rates: Vec<u32> = vec![];
    if let Some(dev) = host.default_output_device() {
        if let Ok(cfgs) = dev.supported_output_configs() {
            for range in cfgs {
                if range.sample_format() == cpal::SampleFormat::I16 {
                    output_rates.push(range.min_sample_rate().0);
                    output_rates.push(range.max_sample_rate().0);
                    break;
                }
            }
        }
    }
    serde_json::json!({
        "host": host_name,
        "input_device": input_name,
        "output_device": output_name,
        "input_rates": input_rates,
        "output_rates": output_rates,
    })
}

#[cfg(feature = "realtime-audio")]
pub async fn vad_capture_diagnostic(seconds: u32, desired_sr: u32, chunk_ms: u32, sensitivity: f32, min_speech_ms: u32, transcribe: bool) -> serde_json::Value {
    use tokio::sync::mpsc;
    // Gather energies and simple VAD stats
    let (tx, mut rx) = mpsc::channel::<Vec<i16>>(32);
    let mut opened = false;
    let mut error: Option<String> = None;
    let device_name = cpal::default_host().default_input_device().and_then(|d| d.name().ok());
    // Hold non-Send AudioCapture on a dedicated thread for the test duration
    let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let stop_t = stop.clone();
    let (started_tx, started_rx) = std::sync::mpsc::channel::<Result<(), String>>();
    std::thread::spawn(move || {
        match start_capture(desired_sr, chunk_ms, tx) {
            Ok(cap) => {
                let _ = started_tx.send(Ok(()));
                while !stop_t.load(std::sync::atomic::Ordering::SeqCst) {
                    std::thread::sleep(std::time::Duration::from_millis(50));
                }
                drop(cap);
            }
            Err(e) => {
                let _ = started_tx.send(Err(e.to_string()));
            }
        }
    });
    match started_rx.recv_timeout(std::time::Duration::from_secs(2)) {
        Ok(Ok(())) => { opened = true; }
        Ok(Err(e)) => { error = Some(e); }
        Err(_timeout) => { error = Some("capture start timeout".into()); }
    }
    // Fixed VAD threshold per user request
    let mut last_thr: f32 = 0.010;
    let mut total_samples: usize = 0;
    let mut frames: u32 = 0;
    let mut energies: Vec<f32> = vec![];
    let mut avg_energy: f32 = 0.0;
    let mut peak_energy: f32 = 0.0;
    let mut in_speech = false;
    let mut cur_ms: u32 = 0;
    let mut speech_ms_total: u32 = 0;
    let mut speech_segments: u32 = 0;
    let mut seg_buf: Vec<i16> = vec![];
    let mut last_segment: Option<Vec<i16>> = None;

    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(seconds as u64);
    while std::time::Instant::now() < deadline {
        if let Some(frame) = tokio::time::timeout(std::time::Duration::from_millis((chunk_ms + 10) as u64), rx.recv()).await.ok().flatten() {
            frames += 1;
            total_samples += frame.len();
            let e = frame.iter().map(|s| (s.abs() as f32) / 32768.0).sum::<f32>() / (frame.len().max(1) as f32);
            energies.push(e);
            avg_energy += e;
            if e > peak_energy { peak_energy = e; }
            let thr: f32 = 0.010;
            last_thr = thr;
            if e > thr {
                in_speech = true; cur_ms += chunk_ms; speech_ms_total += chunk_ms; seg_buf.extend_from_slice(&frame);
            } else {
                if in_speech {
                    if cur_ms >= min_speech_ms { speech_segments += 1; last_segment = Some(std::mem::take(&mut seg_buf)); }
                    in_speech = false; cur_ms = 0; seg_buf.clear();
                }
            }
        } else {
            // timeout - allow loop to continue until deadline
        }
    }
    if in_speech && cur_ms >= min_speech_ms { speech_segments += 1; last_segment = Some(seg_buf); }
    if frames > 0 { avg_energy /= frames as f32; }
    // noise floor approx: average of first 1s worth of frames
    let frames_per_sec = (1000 / chunk_ms.max(1)) as usize;
    let nf = if energies.is_empty() { 0.0 } else { energies.iter().take(frames_per_sec).copied().sum::<f32>() / (energies.len().min(frames_per_sec).max(1) as f32) };

    let mut result = serde_json::json!({
        "opened": opened,
        "error": error,
        "input_device": device_name,
        "sample_rate": desired_sr,
        "chunk_ms": chunk_ms,
        "frames": frames,
        "samples": total_samples as u64,
        "avg_energy": avg_energy,
        "peak_energy": peak_energy,
        "noise_floor": nf,
        "vad_threshold": last_thr,
        "speech_segments": speech_segments,
        "speech_ms_total": speech_ms_total,
    });

    if transcribe {
        if let Some(pcm) = last_segment {
            if std::env::var("OPENAI_API_KEY").is_ok() {
                if let Ok(text) = crate::stt::transcribe_openai_pcm16(&pcm, desired_sr).await {
                    result.as_object_mut().unwrap().insert("transcript".into(), serde_json::json!(text));
                } else {
                    result.as_object_mut().unwrap().insert("transcript".into(), serde_json::json!("(stt error)"));
                }
            } else {
                result.as_object_mut().unwrap().insert("transcript".into(), serde_json::json!("(OPENAI_API_KEY not set)"));
            }
        } else {
            result.as_object_mut().unwrap().insert("transcript".into(), serde_json::json!("(no segment)"));
        }
    }

    // Signal stop and allow the capture thread to exit
    stop.store(true, std::sync::atomic::Ordering::SeqCst);
    result
}

#[cfg(feature = "realtime-audio")]
pub fn play_beep(seconds: u32, sr: u32, freq_hz: f32) -> anyhow::Result<()> {
    let pb = AudioPlayback::new(sr)?;
    let chunk_ms = 30u32;
    let total_chunks = (seconds * 1000) / chunk_ms;
    let samples_per_chunk = (sr as usize) * (chunk_ms as usize) / 1000;
    let mut phase: f32 = 0.0;
    let phase_inc = (2.0 * std::f32::consts::PI * freq_hz) / (sr as f32);
    for _ in 0..total_chunks {
        let mut pcm: Vec<i16> = Vec::with_capacity(samples_per_chunk);
        for _ in 0..samples_per_chunk {
            let s = (phase.sin() * 0.2 * (i16::MAX as f32)) as i16;
            pcm.push(s);
            phase += phase_inc;
            if phase > 2.0 * std::f32::consts::PI { phase -= 2.0 * std::f32::consts::PI; }
        }
        pb.push_pcm(&pcm, sr);
        std::thread::sleep(std::time::Duration::from_millis(chunk_ms as u64));
    }
    Ok(())
}
