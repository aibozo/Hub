use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::oneshot;
use std::collections::VecDeque;
use futures_util::{SinkExt, StreamExt};
use std::path::PathBuf;
use uuid::Uuid;
#[cfg(feature = "realtime")]
use tokio::net::TcpStream;
#[cfg(feature = "realtime")]
use tokio_tungstenite::MaybeTlsStream;
#[cfg(feature = "realtime-audio")]
use tokio::sync::mpsc;
#[cfg(feature = "realtime-audio")]
use base64::Engine as _;
#[cfg(feature = "realtime-audio")]
use base64::engine::general_purpose::STANDARD as B64;

fn rt_log(line: impl AsRef<str>) {
    let line = line.as_ref();
    let _ = std::fs::create_dir_all("storage/logs");
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open("storage/logs/realtime.log") {
        use std::io::Write as _;
        let ts = match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) { Ok(d) => d.as_millis(), Err(_) => 0 };
        let _ = writeln!(f, "[{}] {}", ts, line);
    }
}

#[cfg(feature = "realtime")]
async fn handle_ws_message(
    maybe_msg: Option<Result<tokio_tungstenite::tungstenite::Message, tokio_tungstenite::tungstenite::Error>>,
    ws: &mut tokio_tungstenite::WebSocketStream<MaybeTlsStream<TcpStream>>,
    inner: &Arc<RwLock<InnerState>>,
    tools: &crate::tools::ToolsManager,
    policy: &Arc<crate::gatekeeper::PolicyEngine>,
    approval_prompt: &Arc<RwLock<Option<crate::app::EphemeralApproval>>>,
    chat_dir: &Option<PathBuf>,
    in_sr: u32,
    out_fmt: &String,
    #[allow(unused_variables)] playback: Option<&crate::realtime_audio::AudioPlayback>,
) {
    use tokio_tungstenite::tungstenite::Message;
    match maybe_msg {
        Some(Ok(Message::Text(txt))) => {
            rt_log(format!("<- TEXT {} bytes", txt.len()));
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&txt) {
                let typ = v.get("type").and_then(|s| s.as_str()).unwrap_or("");
                if !typ.is_empty() { rt_log(format!("<- event: {}", typ)); }
                #[cfg(feature = "realtime-audio")]
                if typ.ends_with("audio.delta") || typ.ends_with("output_audio.delta") {
                    // Accept either {audio:"..."} or {delta:"..."}
                    let audio_field = v.get("audio").and_then(|s| s.as_str()).or_else(|| v.get("delta").and_then(|s| s.as_str()));
                    if let Some(b64) = audio_field {
                        rt_log(format!("<- audio.delta len(b64)={}", b64.len()));
                        if let Ok(bytes) = B64.decode(b64) {
                            let mut pcm = if out_fmt == "g711_ulaw" { crate::realtime_audio::decode_ulaw_to_pcm(&bytes) } else {
                                let mut v2 = Vec::with_capacity(bytes.len()/2);
                                for chunk in bytes.chunks_exact(2) { v2.push(i16::from_le_bytes([chunk[0], chunk[1]])); }
                                v2
                            };
                            let samples = pcm.len();
                            let ms = (samples as f32) * 1000.0 / (in_sr as f32);
                            rt_log(format!("   decoded {} bytes -> {} samples (~{:.1} ms @ {} Hz)", bytes.len(), samples, ms, in_sr));
                            // Mark that we are currently playing assistant audio
                            {
                                let mut g = inner.write();
                                if !g.playing_audio { rt_log("[state] assistant speaking (delta)" ); }
                                g.playing_audio = true;
                            }
                            // Apply playback gain (default 0.25)
                            let gain: f32 = std::env::var("REALTIME_PLAYBACK_GAIN").ok().and_then(|s| s.parse::<f32>().ok()).unwrap_or(0.25).clamp(0.0, 1.0);
                            if gain != 1.0 { for s in pcm.iter_mut() { let v = (*s as f32) * gain; *s = v.clamp(i16::MIN as f32, i16::MAX as f32) as i16; } }
                            if let Some(pb) = playback { pb.push_pcm(&pcm, in_sr); }
                            // Accumulate assistant audio for possible STT
                            {
                                let mut g = inner.write();
                                g.assistant_pcm.extend_from_slice(&pcm);
                            }
                        }
                    }
                    return;
                }
                // Accumulate assistant text deltas; flush on response.done
                if typ.ends_with("output_text.delta") || typ.ends_with("text.delta") || typ == "response.output_text.delta" || typ == "response.text.delta" {
                    if let Some(delta) = v.get("delta").and_then(|s| s.as_str()).or_else(|| v.get("text").and_then(|s| s.as_str())) {
                        let mut g = inner.write();
                        g.assistant_text_buf.push_str(delta);
                    }
                    return;
                }
                // Also support audio transcript events when server streams them
                if typ.ends_with("audio_transcript.delta") || typ == "response.audio_transcript.delta" {
                    if let Some(delta) = v.get("delta").and_then(|s| s.as_str()).or_else(|| v.get("text").and_then(|s| s.as_str())) {
                        let mut g = inner.write();
                        g.assistant_text_buf.push_str(delta);
                    }
                    return;
                }
                if typ.ends_with("output_text.done") || typ.ends_with("text.done") || typ.ends_with("audio_transcript.done") || typ.ends_with("response.done") || typ == "response.completed" {
                    // Flush assistant text buffer to chat
                    let text = {
                        let mut g = inner.write();
                        if g.assistant_text_buf.is_empty() { None } else { Some(std::mem::take(&mut g.assistant_text_buf)) }
                    };
                    if let (Some(dir), Some(line)) = (chat_dir.as_ref(), text) {
                        let _ = append_latest_chat_message(dir, "assistant", &line).await;
                    }
                    // Also mark end of speaking handled below for audio; continue
                }
                // Toggle playing state based on lifecycle events
                if typ.ends_with("response.audio.done") || typ.ends_with("response.done") {
                    let mut g = inner.write();
                    if g.playing_audio { rt_log("[state] assistant done speaking"); }
                    g.playing_audio = false;
                    // If no text was flushed, attempt STT on assistant audio for chat logging
                    let have_text = !g.assistant_text_buf.is_empty();
                    let pcm: Vec<i16> = if have_text { vec![] } else { std::mem::take(&mut g.assistant_pcm) };
                    drop(g);
                    if !have_text && !pcm.is_empty() {
                        if let Some(dir) = chat_dir.as_ref() {
                            let dir = dir.clone();
                            tokio::spawn(async move {
                                let t = transcribe_user_local(&pcm, in_sr).await; // reuse same STT
                                if !t.is_empty() { let _ = append_latest_chat_message(&dir, "assistant", &t).await; }
                            });
                        }
                    }
                    return;
                }
                if typ.ends_with("response.created") || typ.ends_with("response.output_item.added") {
                    let mut g = inner.write();
                    if !g.playing_audio { rt_log("[state] response started"); }
                    g.playing_audio = true;
                    g.assistant_pcm.clear();
                    // fall through to normal processing/logging
                }
                if typ == "input_audio_buffer.speech_started" || typ.ends_with("speech_started") {
                    rt_log("<- speech_started");
                    // Mark that a user utterance started (no-op for now; ring always accumulates)
                }
                // Server-side user transcription (preferred): accumulate and flush
                if (typ.contains("input_audio_buffer") && typ.contains("transcription") && typ.ends_with("delta")) || typ == "input_audio_buffer.transcription.delta" {
                    if let Some(delta) = v.get("delta").and_then(|s| s.as_str()).or_else(|| v.get("text").and_then(|s| s.as_str())).or_else(|| v.get("transcript").and_then(|s| s.as_str())) {
                        let mut g = inner.write();
                        g.user_text_buf.push_str(delta);
                    }
                    return;
                }
                if (typ.contains("input_audio_buffer") && typ.contains("transcription") && (typ.ends_with("completed") || typ.ends_with("done"))) || typ == "input_audio_buffer.transcription.completed" {
                    let text = {
                        let mut g = inner.write();
                        if g.user_text_buf.is_empty() { None } else { Some(std::mem::take(&mut g.user_text_buf)) }
                    };
                    if let (Some(dir), Some(line)) = (chat_dir.as_ref(), text) {
                        let _ = append_latest_chat_message(dir, "user", &line).await;
                    }
                    return;
                }
                if typ == "error" || typ.ends_with("error") {
                    // Try to extract nested error.message or log raw JSON
                    let msg = v.get("error")
                        .and_then(|e| e.get("message").and_then(|m| m.as_str()))
                        .or_else(|| v.get("message").and_then(|m| m.as_str()))
                        .unwrap_or("");
                    if !msg.is_empty() { rt_log(format!("<- error: {}", msg)); }
                    else { rt_log(format!("<- error raw: {}", txt)); }
                    let mut g = inner.write();
                    g.status.last_error = Some(if !msg.is_empty() { format!("realtime error: {}", msg) } else { "realtime error".into() });
                    return;
                }
                if typ == "input_audio_buffer.speech_stopped" || typ.ends_with("speech_stopped") {
                    // Server VAD signaled end of user turn; request a response
                    rt_log("<- speech_stopped; -> response.create [audio,text]");
                    let create = serde_json::json!({"type":"response.create","response": {"modalities":["audio","text"]}});
                    let _ = ws.send(Message::Text(create.to_string())).await;
                    // Also produce a local transcript for chat logging (best-effort)
                    if let Some(dir) = chat_dir.as_ref() {
                        let samples: Vec<i16> = {
                            let g = inner.read();
                            let take = (in_sr as usize) * 4; // last 4s
                            let n = g.ring.len();
                            let start = n.saturating_sub(take);
                            g.ring.iter().skip(start).cloned().collect()
                        };
                        if !samples.is_empty() {
                            let dir = dir.clone();
                            tokio::spawn(async move {
                                let t = transcribe_user_local(&samples, in_sr).await;
                                if !t.is_empty() { let _ = append_latest_chat_message(&dir, "user", &t).await; }
                            });
                        }
                    }
                    return;
                }
                if typ == "tool.call" || typ == "tool_call" {
                    let id = v.get("id").and_then(|s| s.as_str()).map(|s| s.to_string());
                    let name = v.get("name").and_then(|s| s.as_str()).unwrap_or("").to_string();
                    let args_val = v.get("arguments").cloned().unwrap_or_else(|| serde_json::json!({}));
                    let args = if args_val.is_string() { serde_json::from_str(args_val.as_str().unwrap_or("{}" )).unwrap_or_else(|_| serde_json::json!({})) } else { args_val };

                    // Special case: end_call terminates the session
                    if name == "end_call" || name == "end.call" {
                        let mut out = serde_json::json!({"type": "tool.output", "name": name, "output": {"ok": true}});
                        if let Some(i) = id { out.as_object_mut().unwrap().insert("id".into(), serde_json::json!(i)); }
                        let _ = ws.send(Message::Text(out.to_string())).await;
                        // Append a brief end-of-call summary to latest chat (best-effort)
                        if let Some(dir) = chat_dir.as_ref() {
                            if let Some(sum) = build_session_summary(inner, chat_dir.as_ref()) { let _ = append_voice_summary(dir, &sum).await; }
                        }
                        { let mut g = inner.write(); g.session_log = None; g.status.active = false; }
                        let _ = ws.close(None).await;
                        return;
                    }

                    let result = handle_tool_call(tools, policy, approval_prompt, &name, args.clone()).await;
                    // Update session log
                    {
                        let ok = !result.get("error").is_some();
                        let err = result.get("error").and_then(|e| e.as_str()).map(|s| truncate(s, 120));
                        let mut g = inner.write();
                        if let Some(log) = g.session_log.as_mut() {
                            log.tool_calls.push(SessionToolEvent { name: name.clone(), ok, error: err });
                        }
                    }
                    let mut out = serde_json::json!({"type": "tool.output", "name": name, "output": result});
                    if let Some(i) = id { out.as_object_mut().unwrap().insert("id".into(), serde_json::json!(i)); }
                    let _ = ws.send(Message::Text(out.to_string())).await;
                }
            }
        }
        Some(Ok(Message::Binary(bin))) => {
            rt_log(format!("<- BINARY {} bytes", bin.len()));
            #[cfg(feature = "realtime-audio")]
            if let Some(pb) = playback {
                let pcm = if out_fmt == "g711_ulaw" { crate::realtime_audio::decode_ulaw_to_pcm(&bin) } else {
                    let mut v2 = Vec::with_capacity(bin.len()/2);
                    for chunk in bin.chunks_exact(2) { v2.push(i16::from_le_bytes([chunk[0], chunk[1]])); }
                    v2
                };
                let samples = pcm.len();
                let ms = (samples as f32) * 1000.0 / (in_sr as f32);
                rt_log(format!("   binary audio -> {} samples (~{:.1} ms @ {} Hz)", samples, ms, in_sr));
                // Mark that we are currently playing assistant audio
                {
                    let mut g = inner.write();
                    if !g.playing_audio { rt_log("[state] assistant speaking (binary)"); }
                    g.playing_audio = true;
                }
                pb.push_pcm(&pcm, in_sr);
                // Accumulate assistant audio for possible STT
                {
                    let mut g = inner.write();
                    g.assistant_pcm.extend_from_slice(&pcm);
                }
            }
        }
        Some(Ok(Message::Close(c))) => { rt_log(format!("<- CLOSE: {:?}", c)); let _ = ws.close(None).await; let mut g = inner.write(); g.status.active = false; }
        None => { rt_log("<- CLOSE: None"); let _ = ws.close(None).await; let mut g = inner.write(); g.status.active = false; }
        _ => {}
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RealtimeAudioOpts {
    #[serde(default)]
    pub in_sr: Option<u32>,
    #[serde(default)]
    pub out_format: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RealtimeOptions {
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub voice: Option<String>,
    #[serde(default)]
    pub audio: Option<RealtimeAudioOpts>,
    #[serde(default)]
    pub instructions: Option<String>,
    #[serde(default)]
    pub endpoint: Option<String>,
    #[serde(default)]
    pub transport: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RealtimeStatus {
    pub active: bool,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub since: Option<DateTime<Utc>>,
    #[serde(default)]
    pub last_error: Option<String>,
}

#[derive(Default)]
struct InnerState {
    status: RealtimeStatus,
    #[allow(dead_code)]
    handle: Option<std::thread::JoinHandle<()>>,
    stop_tx: Option<oneshot::Sender<()>>,
    session_log: Option<SessionLog>,
    playing_audio: bool,
    // Accumulators for chat logging
    user_text_buf: String,
    assistant_text_buf: String,
    // Accumulate assistant PCM for fallback STT of agent replies
    assistant_pcm: Vec<i16>,
    // Recent mic audio ring buffer for local STT
    ring: VecDeque<i16>,
}

#[derive(Clone)]
pub struct RealtimeManager {
    inner: Arc<RwLock<InnerState>>,
    tools: crate::tools::ToolsManager,
    policy: Arc<crate::gatekeeper::PolicyEngine>,
    approval_prompt: Arc<RwLock<Option<crate::app::EphemeralApproval>>>,
    chat_dir: Option<PathBuf>,
}

impl Default for RealtimeManager {
    fn default() -> Self {
        Self {
            inner: Arc::new(RwLock::new(InnerState { status: RealtimeStatus::default(), handle: None, stop_tx: None, session_log: None, playing_audio: false, user_text_buf: String::new(), assistant_text_buf: String::new(), assistant_pcm: Vec::new(), ring: VecDeque::with_capacity(16000*8) })),
            tools: crate::tools::ToolsManager::default(),
            policy: Arc::new(crate::gatekeeper::PolicyEngine::default()),
            approval_prompt: Arc::new(RwLock::new(None)),
            chat_dir: None,
        }
    }
}

impl RealtimeManager {
    pub fn new(tools: crate::tools::ToolsManager, policy: Arc<crate::gatekeeper::PolicyEngine>, approval_prompt: Arc<RwLock<Option<crate::app::EphemeralApproval>>>, chat_dir: Option<PathBuf>) -> Self {
        Self {
            inner: Arc::new(RwLock::new(InnerState { status: RealtimeStatus::default(), handle: None, stop_tx: None, session_log: None, playing_audio: false, user_text_buf: String::new(), assistant_text_buf: String::new(), assistant_pcm: Vec::new(), ring: VecDeque::with_capacity(16000*8) })),
            tools,
            policy,
            approval_prompt,
            chat_dir,
        }
    }

    pub async fn start(&self, opts: RealtimeOptions) -> anyhow::Result<()> {
        #[cfg(not(feature = "realtime"))]
        {
            anyhow::bail!("realtime feature disabled");
        }

        #[cfg(feature = "realtime")]
        {
            // Prepare configuration and update status before spawning
            let mut g = self.inner.write();
            if g.status.active { anyhow::bail!("realtime already active"); }
            let model = opts.model.clone().unwrap_or_else(|| "gpt-realtime".to_string());
            g.status.model = Some(model.clone());
            g.status.last_error = None;
            drop(g);

            // Extract options to move into the async task
                let mut endpoint = opts.endpoint.unwrap_or_else(|| "wss://api.openai.com/v1/realtime?model=gpt-realtime".to_string());
                if !endpoint.starts_with("ws://") && !endpoint.starts_with("wss://") {
                    endpoint = format!("ws://{}", endpoint);
                }
            let voice = opts.voice.unwrap_or_else(|| "alloy".into());
            // Capture sample rate (default 16 kHz), overridable via opts.audio.in_sr
            let cap_sr: u32 = opts.audio.as_ref().and_then(|a| a.in_sr).unwrap_or(16_000);
            let out_fmt = opts.audio.as_ref().and_then(|a| a.out_format.clone()).unwrap_or_else(|| "pcm16".into());
            // Choose server output sample rate based on format (can differ by codec)
            // - pcm16: typically 24 kHz from Realtime
            // - g711_ulaw: telephony-grade 8 kHz
            let srv_out_sr: u32 = match out_fmt.as_str() {
                "g711_ulaw" => 8_000,
                _ => 24_000,
            };
            let base_instructions = opts.instructions.clone();
            let transport = opts.transport.clone().unwrap_or_else(|| std::env::var("OPENAI_REALTIME_TRANSPORT").unwrap_or_else(|_| "websocket".into()));

            // spawn connection task
            let (tx, mut rx) = oneshot::channel::<()>();
            let inner = self.inner.clone();
            let tools = self.tools.clone();
            let policy = self.policy.clone();
            let approval_prompt = self.approval_prompt.clone();
            let chat_dir = self.chat_dir.clone();
            let handle = std::thread::spawn(move || {
                let rt = tokio::runtime::Builder::new_current_thread().enable_all().build();
                if rt.is_err() { return; }
                let rt = rt.unwrap();
                rt.block_on(async move {
                let mut instructions = base_instructions.unwrap_or_else(|| crate::prompt::base_system_prompt());
                instructions.push_str(&crate::prompt::voice_mode_suffix());
                // Seed with recent chat turns (compact digest)
                if let Some(dir) = chat_dir.as_ref() {
                    if let Some(ctx) = build_recent_context(dir).await { instructions.push_str("\n\nRecent context:\n"); instructions.push_str(&ctx); }
                }

                // Transport selection (default websocket; optional webrtc stub)
                if transport.eq_ignore_ascii_case("webrtc") {
                    let mut g = inner.write();
                    g.status.last_error = Some("realtime transport=webrtc not implemented in this build".into());
                    g.status.active = false;
                    return;
                }

                // WebSocket connect (with optional auth headers)
                rt_log(format!("connecting to {}", endpoint));
                use tokio_tungstenite::tungstenite::client::IntoClientRequest;
                let mut req = match endpoint.clone().into_client_request() {
                    Ok(r) => r,
                    Err(e) => {
                        rt_log(format!("bad endpoint: {}", e));
                        let mut g = inner.write();
                        g.status.last_error = Some(format!("bad endpoint {}: {}", endpoint, e));
                        g.status.active = false; return;
                    }
                };
                // If OPENAI_API_KEY is present or endpoint looks like OpenAI, attach headers
                if std::env::var("OPENAI_API_KEY").is_ok() || endpoint.contains("api.openai.com") {
                    use hyper::header::HeaderValue;
                    if let Ok(key) = std::env::var("OPENAI_API_KEY") {
                        let headers = req.headers_mut();
                        headers.insert("Authorization", HeaderValue::from_str(&format!("Bearer {}", key)).unwrap_or(HeaderValue::from_static("")));
                        headers.insert("OpenAI-Beta", HeaderValue::from_static("realtime=v1"));
                        headers.insert("Sec-WebSocket-Protocol", HeaderValue::from_static("openai-realtime-v1"));
                    }
                }
                let (mut ws, _resp) = match tokio_tungstenite::connect_async(req).await {
                    Ok(ok) => {
                        rt_log("connected");
                        let mut g = inner.write();
                        g.status.active = true;
                        g.status.since = Some(Utc::now());
                        g.session_log = Some(SessionLog { started_at: Utc::now(), tool_calls: vec![] });
                        ok
                    }
                    Err(e) => {
                        rt_log(format!("connect error: {}", e));
                        let mut g = inner.write();
                        g.status.last_error = Some(format!("connect {}: {}", endpoint, e));
                        g.status.active = false;
                        return;
                    }
                };

                #[cfg(feature = "realtime-audio")]
                let playback = match crate::realtime_audio::AudioPlayback::new(srv_out_sr) { Ok(pb) => { rt_log(format!("[cfg] playback_device_sr={}Hz", pb.device_sr())); Some(pb) }, Err(e) => { eprintln!("[realtime] audio playback init error: {}", e); None } };
                #[cfg(feature = "realtime-audio")]
                let (tx_frames, mut rx_frames) = mpsc::channel::<Vec<i16>>(8);
                #[cfg(feature = "realtime-audio")]
                let _cap_keepalive = match crate::realtime_audio::start_capture(cap_sr, 40, tx_frames) { Ok(c) => Some(c), Err(e) => { eprintln!("[realtime] audio capture init error: {}", e); None } };

                // Build session.update with fields supported by OpenAI Realtime WS
                let payload = serde_json::json!({
                    "type": "session.update",
                    "session": {
                        "model": model,
                        "instructions": instructions,
                        "voice": voice,
                        "modalities": ["audio","text"],
                        "input_audio_format": "pcm16",
                        "output_audio_format": out_fmt,
                        "turn_detection": { "type": "server_vad" },
                        "tools": crate::tools::realtime_tool_schemas(&tools)
                    }
                });

                use tokio_tungstenite::tungstenite::Message;
                let upd_txt = payload.to_string();
                // Log negotiated audio parameters for quick sanity checks
                rt_log(format!(
                    "[cfg] capture_sr={}Hz, capture_chunk_ms={}, output_sr={}Hz, out_fmt={}",
                    cap_sr, 40, srv_out_sr, out_fmt
                ));
                // Log a short preview of instructions for debugging
                let preview = if upd_txt.len() > 0 {
                    // try to pull the instructions value length and head
                    match serde_json::from_str::<serde_json::Value>(&upd_txt) {
                        Ok(v) => v.get("session").and_then(|s| s.get("instructions")).and_then(|i| i.as_str()).map(|t| {
                            let mut head = t.to_string();
                            if head.len() > 120 { head.truncate(120); head.push('…'); }
                            format!("(instr {} chars) {}", t.len(), head)
                        }).unwrap_or_default(),
                        Err(_) => String::new(),
                    }
                } else { String::new() };
                rt_log(format!("-> session.update {} bytes {}", upd_txt.len(), preview));
                if ws.send(Message::Text(upd_txt)).await.is_err() {
                    let mut g = inner.write();
                    g.status.last_error = Some("send session.update failed".into());
                    g.status.active = false;
                    rt_log("session.update send failed");
                    return;
                }

                // Event loop: stop on signal or socket close. We stream audio frames continuously; server VAD will commit turns.
                #[cfg(feature = "realtime-audio")]
                {
                    loop {
                        tokio::select! {
                            _ = tokio::time::sleep(std::time::Duration::from_millis(10)) => {},
                            Some(frame) = rx_frames.recv() => {
                                let pcm = frame;
                                // Push into recent ring for local STT (keep ~8s)
                                {
                                    let mut g = inner.write();
                                    for s in &pcm { g.ring.push_back(*s); }
                                    let max_samples = (cap_sr as usize) * 8;
                                    while g.ring.len() > max_samples { g.ring.pop_front(); }
                                }
                                // Half-duplex: if assistant is speaking, drop mic frames to avoid barge-in
                                if inner.read().playing_audio {
                                    rt_log("(drop) mic frame during playback");
                                } else {
                                    let frame_samples = pcm.len();
                                    let frame_ms = (frame_samples as f32) * 1000.0 / (cap_sr as f32);
                                    let mut bytes = Vec::with_capacity(frame_samples*2);
                                    for s in pcm { bytes.extend_from_slice(&s.to_le_bytes()); }
                                    let audio_b64 = B64.encode(&bytes);
                                    rt_log(format!(
                                        "-> input_audio_buffer.append b64_len={} ({} samples, ~{:.1} ms @ {} Hz)",
                                        audio_b64.len(), frame_samples, frame_ms, cap_sr
                                    ));
                                    let event = serde_json::json!({"type":"input_audio_buffer.append","audio": audio_b64});
                                    let _ = ws.send(tokio_tungstenite::tungstenite::Message::Text(event.to_string())).await;
                                }
                            },
                            maybe_msg = ws.next() => {
                                handle_ws_message(maybe_msg, &mut ws, &inner, &tools, &policy, &approval_prompt, &chat_dir, srv_out_sr, &out_fmt, playback.as_ref()).await;
                                if !inner.read().status.active { break; }
                            }
                            _ = &mut rx => {
                                let _ = ws.close(None).await; break;
                            }
                        }
                    }
                }
                #[cfg(not(feature = "realtime-audio"))]
                {
                    loop {
                        tokio::select! {
                            _ = tokio::time::sleep(std::time::Duration::from_millis(10)) => {},
                            maybe_msg = ws.next() => {
                                handle_ws_message(maybe_msg, &mut ws, &inner, &tools, &policy, &approval_prompt, &chat_dir, 24_000, &"pcm16".to_string(), None).await;
                                if !inner.read().status.active { break; }
                            }
                            _ = &mut rx => { let _ = ws.close(None).await; break; }
                        }
                    }
                }

                let mut g = inner.write();
                g.status.active = false;
                });
            });

            {
                let mut g = self.inner.write();
                g.stop_tx = Some(tx);
                g.handle = Some(handle);
            }
            Ok(())
        }
    }

    pub async fn stop(&self) -> anyhow::Result<()> {
        let (stop_tx, handle_opt) = {
            let mut g = self.inner.write();
            (g.stop_tx.take(), g.handle.take())
        };
        if let Some(tx) = stop_tx { let _ = tx.send(()); }
        if let Some(handle) = handle_opt { let _ = handle.join(); }
        let mut g = self.inner.write();
        g.status.active = false;
        Ok(())
    }

    pub fn status(&self) -> RealtimeStatus {
        self.inner.read().status.clone()
    }
}

#[allow(unused)]
async fn handle_tool_call(tools: &crate::tools::ToolsManager, policy: &crate::gatekeeper::PolicyEngine, approval_prompt: &Arc<RwLock<Option<crate::app::EphemeralApproval>>>, name: &str, args: serde_json::Value) -> serde_json::Value {
    // Split server.tool or server_tool
    let (server, tool) = if let Some((s, t)) = name.split_once('.') { (s.to_string(), t.to_string()) }
                        else if let Some((s,t)) = name.split_once('_') { (s.to_string(), t.to_string()) }
                        else { (name.to_string(), String::new()) };
    // Special synthetic tool: end_call
    if server == "end" && tool == "call" || name == "end_call" {
        // Caller (model) is asking to end; return a simple ack. The bridge will interpret this and stop shortly after.
        return serde_json::json!({"ok": true});
    }
    // Basic policy evaluation, with ephemeral approval prompt on Hold/Warn
    let action = crate::gatekeeper::ProposedAction { command: format!("{}.{}", server, tool), writes: might_write(&server, &tool), paths: vec![], intent: None };
    let mut decision = policy.evaluate(&action);
    if decision.kind != crate::gatekeeper::PolicyDecisionKind::Allow {
        // Prepare or wait for an ephemeral prompt
        // If another prompt exists, wait for it to clear; else set ours and wait.
        let my_id = uuid::Uuid::new_v4().to_string();
        let title = format!("Realtime tool requires approval: {}.{}", server, tool);
        let details = serde_json::json!({"server": server, "tool": tool, "arguments": args.clone()});
        {
            let mut w = approval_prompt.write();
            if w.is_none() {
                *w = Some(crate::app::EphemeralApproval { id: my_id.clone(), title, action: action.clone(), details });
            }
        }
        // Wait until prompt clears (treated as approved). Timeout after 120s.
        let start = std::time::Instant::now();
        loop {
            if start.elapsed().as_secs() > 120 {
                return serde_json::json!({"error": "approval timeout"});
            }
            let cur = approval_prompt.read().clone();
            match cur {
                None => break,
                Some(p) => {
                    if p.id != my_id { /* another prompt is active; keep waiting */ }
                }
            }
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        }
        // Re-evaluate (policy may still be non-allow but we proceed as approved for now)
        decision = policy.evaluate(&action);
        // Continue regardless; approvals path accepted
    }
    // Invoke tool
    match tools.invoke(&server, &tool, args).await {
        Ok(v) => v,
        Err(e) => serde_json::json!({"error": e.to_string()})
    }
}

fn might_write(server: &str, tool: &str) -> bool {
    if server == "installer" { return true; }
    if tool.contains("write") || tool.contains("apply") || tool.contains("install") { return true; }
    false
}

#[derive(Clone, Debug)]
struct SessionToolEvent { name: String, ok: bool, error: Option<String> }

#[derive(Clone, Debug)]
struct SessionLog { started_at: DateTime<Utc>, tool_calls: Vec<SessionToolEvent> }

fn build_session_summary(inner: &Arc<RwLock<InnerState>>, chat_dir: Option<&PathBuf>) -> Option<String> {
    let (started, calls) = {
        let g = inner.read();
        let log = g.session_log.as_ref()?;
        (log.started_at, log.tool_calls.clone())
    };
    let dur = Utc::now().signed_duration_since(started).num_seconds();
    let mut lines = vec![format!("(voice) Session ended — duration {}s, tool calls {}", dur.max(0), calls.len())];

    // Optional: last user utterance from latest chat
    if let Some(dir) = chat_dir {
        if let Some(last_user) = last_user_utterance(dir) { lines.push(format!("Last user: {}", truncate(&last_user, 160))); }
    }

    for ev in calls.iter().take(10) {
        if ev.ok { lines.push(format!("- {}: ok", ev.name)); }
        else { lines.push(format!("- {}: error: {}", ev.name, ev.error.clone().unwrap_or_else(|| "(error)".into()))); }
    }
    if calls.len() > 10 { lines.push(format!("- … {} more", calls.len() - 10)); }

    // Totals per tool
    use std::collections::BTreeMap;
    let mut totals: BTreeMap<String, (u32, u32, u32)> = BTreeMap::new(); // name -> (total, ok, err)
    for ev in calls {
        let entry = totals.entry(ev.name).or_insert((0,0,0));
        entry.0 += 1;
        if ev.ok { entry.1 += 1; } else { entry.2 += 1; }
    }
    if !totals.is_empty() {
        let mut agg: Vec<String> = vec![];
        for (name, (t, ok, err)) in totals.into_iter() { agg.push(format!("{} x{} ok={} err={}", name, t, ok, err)); }
        lines.push(format!("Totals: {}", agg.join(", ")));
    }
    Some(lines.join("\n"))
}

fn truncate(s: &str, n: usize) -> String { if s.len() > n { format!("{}…", &s[..n]) } else { s.to_string() } }

async fn build_recent_context(dir: &PathBuf) -> Option<String> {
    // Find latest *.json file
    let mut best: Option<(std::time::SystemTime, PathBuf)> = None;
    if let Ok(mut rd) = tokio::fs::read_dir(dir).await {
        while let Ok(Some(e)) = rd.next_entry().await {
            if e.file_type().await.ok().map(|t| t.is_file()).unwrap_or(false) {
                if e.path().extension().and_then(|s| s.to_str()) == Some("json") {
                    if let Ok(meta) = e.metadata().await { if let Ok(mtime) = meta.modified() { if best.as_ref().map(|(t,_)| mtime > *t).unwrap_or(true) { best = Some((mtime, e.path())); } } }
                }
            }
        }
    }
    let path = best?.1;
    let bytes = tokio::fs::read(&path).await.ok()?;
    let v: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
    let msgs = v.get("messages").and_then(|x| x.as_array())?.clone();
    // Take last 8 messages; compact each to <= 200 chars
    let take = msgs.len().saturating_sub(8);
    let mut lines: Vec<String> = vec![];
    for m in msgs.into_iter().skip(take) {
        let role = m.get("role").and_then(|s| s.as_str()).unwrap_or("");
        let mut content = m.get("content").and_then(|s| s.as_str()).unwrap_or("").to_string();
        if content.len() > 200 { content.truncate(200); content.push_str("…"); }
        if !content.is_empty() { lines.push(format!("{}: {}", role, content)); }
    }
    if lines.is_empty() { None } else { Some(lines.join("\n")) }
}

async fn append_voice_summary(dir: &PathBuf, line: &str) -> anyhow::Result<()> {
    // Append an assistant message to latest chat file
    let mut best: Option<(std::time::SystemTime, PathBuf)> = None;
    if let Ok(mut rd) = tokio::fs::read_dir(dir).await {
        while let Ok(Some(e)) = rd.next_entry().await {
            if e.file_type().await.ok().map(|t| t.is_file()).unwrap_or(false) {
                if e.path().extension().and_then(|s| s.to_str()) == Some("json") {
                    if let Ok(meta) = e.metadata().await { if let Ok(mtime) = meta.modified() { if best.as_ref().map(|(t,_)| mtime > *t).unwrap_or(true) { best = Some((mtime, e.path())); } } }
                }
            }
        }
    }
    let path = match best { Some((_, p)) => p, None => return Ok(()) };
    let mut v: serde_json::Value = match tokio::fs::read(&path).await.ok().and_then(|b| serde_json::from_slice(&b).ok()) { Some(j) => j, None => serde_json::json!({"id":"","messages": []}) };
    let at = chrono::Utc::now().to_rfc3339();
    let msg = serde_json::json!({"role":"assistant","content": line, "at": at});
    v.as_object_mut().and_then(|o| o.get_mut("messages")).and_then(|m| m.as_array_mut()).map(|arr| arr.push(msg));
    let data = serde_json::to_vec_pretty(&v)?;
    tokio::fs::write(&path, data).await?;
    Ok(())
}

async fn append_latest_chat_message(dir: &PathBuf, role: &str, content: &str) -> anyhow::Result<()> {
    // Find latest chat, or create one if none exist
    let mut best: Option<(std::time::SystemTime, PathBuf)> = None;
    if let Ok(mut rd) = tokio::fs::read_dir(dir).await {
        while let Ok(Some(e)) = rd.next_entry().await {
            if e.file_type().await.ok().map(|t| t.is_file()).unwrap_or(false) {
                if e.path().extension().and_then(|s| s.to_str()) == Some("json") {
                    if let Ok(meta) = e.metadata().await { if let Ok(mtime) = meta.modified() { if best.as_ref().map(|(t,_)| mtime > *t).unwrap_or(true) { best = Some((mtime, e.path())); } } }
                }
            }
        }
    }
    let path = match best { Some((_, p)) => p, None => {
        let id = Uuid::new_v4().to_string();
        let p = dir.join(format!("{}.json", id));
        let obj = serde_json::json!({"id": id, "messages": []});
        if let Some(parent) = p.parent() { tokio::fs::create_dir_all(parent).await.ok(); }
        let _ = tokio::fs::write(&p, serde_json::to_vec_pretty(&obj)?).await;
        p
    } };
    let mut v: serde_json::Value = match tokio::fs::read(&path).await.ok().and_then(|b| serde_json::from_slice(&b).ok()) { Some(j) => j, None => serde_json::json!({"id":"","messages": []}) };
    let at = chrono::Utc::now().to_rfc3339();
    let msg = serde_json::json!({"role": role, "content": content, "at": at});
    v.as_object_mut().and_then(|o| o.get_mut("messages")).and_then(|m| m.as_array_mut()).map(|arr| arr.push(msg));
    let data = serde_json::to_vec_pretty(&v)?;
    tokio::fs::write(&path, data).await?;
    Ok(())
}

async fn transcribe_user_local(pcm: &[i16], sr: u32) -> String {
    // Try local voice-daemon STT first, fallback to OpenAI if available
    match crate::stt::transcribe_local_pcm16(pcm, sr, None).await {
        Ok(t) if !t.is_empty() => return t,
        Ok(_) => {},
        Err(e) => { rt_log(format!("stt(local) error: {}", e)); }
    }
    if std::env::var("OPENAI_API_KEY").is_ok() {
        match crate::stt::transcribe_openai_pcm16(pcm, sr).await { Ok(t) => return t, Err(e) => { rt_log(format!("stt(openai) error: {}", e)); } }
    }
    String::new()
}

fn last_user_utterance(dir: &PathBuf) -> Option<String> {
    // Read latest chat and return the last message by role == "user"
    let mut rt = tokio::runtime::Runtime::new().ok()?;
    let res: Option<String> = rt.block_on(async {
        let mut best: Option<(std::time::SystemTime, PathBuf)> = None;
        if let Ok(mut rd) = tokio::fs::read_dir(dir).await {
            while let Ok(Some(e)) = rd.next_entry().await {
                if e.file_type().await.ok().map(|t| t.is_file()).unwrap_or(false) {
                    if e.path().extension().and_then(|s| s.to_str()) == Some("json") {
                        if let Ok(meta) = e.metadata().await { if let Ok(mtime) = meta.modified() { if best.as_ref().map(|(t,_)| mtime > *t).unwrap_or(true) { best = Some((mtime, e.path())); } } }
                    }
                }
            }
        }
        let path = match best { Some((_, p)) => p, None => return None };
        let bytes = tokio::fs::read(&path).await.ok()?;
        let v: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
        let msgs = v.get("messages").and_then(|x| x.as_array())?.clone();
        for m in msgs.into_iter().rev() {
            if m.get("role").and_then(|s| s.as_str()) == Some("user") {
                if let Some(txt) = m.get("content").and_then(|s| s.as_str()) { return Some(txt.to_string()); }
            }
        }
        None
    });
    res
}
