use axum::{
    extract::State,
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::http::StatusCode;
use serde::Serialize;
use std::time::Duration;

use crate::app::SharedState;
use crate::gatekeeper::{Approval, ApprovalStatus, PolicyDecision, PolicyDecisionKind, ProposedAction};
use crate::system_map::SystemMap;
use foreman_memory as fm;
use axum::extract::Path;
use axum::extract::Path as AxPath;
use reqwest::Client as HttpClient;
use futures_util::StreamExt;
use serde::Deserialize;

#[derive(Serialize)]
struct Health {
    status: &'static str,
    version: &'static str,
}

pub fn build_router(state: SharedState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/ready", get(ready))
        .route("/metrics", get(metrics))
        .route("/control", get(ws_upgrade))
        .route("/api/voice/test", get(voice_test))
        // audio diagnostics
        .route("/api/audio/devices", get(audio_devices))
        .route("/api/audio/diagnose", axum::routing::post(|Json(req): Json<AudioDiagReq>| async move {
            #[cfg(feature = "realtime-audio")]
            {
                let v = crate::realtime_audio::vad_capture_diagnostic(req.seconds, req.in_sr, req.chunk_ms, req.sensitivity, req.min_speech_ms, req.transcribe.unwrap_or(false)).await;
                return Json(v).into_response();
            }
            #[cfg(not(feature = "realtime-audio"))]
            {
                return (StatusCode::NOT_IMPLEMENTED, Json(serde_json::json!({"error":"audio disabled"}))).into_response();
            }
        }))
        .route("/api/audio/beep", axum::routing::post(audio_beep))
        .route("/api/chat/complete", axum::routing::post(chat_complete))
        .route("/api/chat/stream", axum::routing::post(chat_stream))
        // realtime voice bridge
        .route("/api/realtime/start", axum::routing::post(realtime_start))
        .route("/api/realtime/stop", axum::routing::post(|state: axum::extract::State<SharedState>| async move { realtime_stop(state).await }))
        .route("/api/realtime/status", get(realtime_status))
        // wake sentinel
        .route("/api/wake/status", get(wake_status))
        .route("/api/wake/enable", axum::routing::post(wake_enable))
        .route("/api/wake/disable", axum::routing::post(wake_disable))
        // codex adapter endpoints
        .route("/api/codex/new", axum::routing::post(codex_new))
        .route("/api/codex/continue", axum::routing::post(codex_continue))
        .route("/api/codex/sessions", get(codex_sessions))
        .route("/api/codex/session/:id", get(codex_session_detail))
        // placeholder API endpoints for wiring in later PRs
        .route("/api/tasks", get(list_tasks).post(create_task))
        .route("/api/system_map", get(system_map))
        .route("/api/system_map/digest", get(system_map_digest))
        .route("/api/system_map/refresh", axum::routing::post(system_map_refresh))
        .route("/api/context/pack", axum::routing::post(context_pack))
        .route("/api/context/expand", axum::routing::post(context_expand))
        // memory APIs
        .route("/api/memory/search", get(memory_search))
        .route("/api/memory/atoms/:id", get(get_atom))
        .route("/api/memory/atoms/:id/pin", axum::routing::post(pin_atom))
        .route("/api/memory/atoms/:id/unpin", axum::routing::post(unpin_atom))
        .route("/api/schedules", get(list_schedules))
        .route("/api/schedules/run/:job", axum::routing::post(run_schedule_job))
        .route("/api/tools", get(list_tools))
        .route("/api/tools/status", get(list_tool_status))
        .route("/api/games", get(list_games))
        .route("/api/approval/prompt", get(get_approval_prompt))
        .route("/api/approval/answer", axum::routing::post(answer_approval))
        .route("/api/approval/explain/:id", get(explain_ephemeral))
        // chat sessions
        .route("/api/chat/sessions", get(chat_sessions_list).post(chat_sessions_create))
        .route("/api/chat/sessions/latest", get(chat_sessions_latest))
        .route("/api/chat/sessions/:id", get(chat_session_get).delete(chat_session_delete))
        .route("/api/chat/sessions/:id/append", axum::routing::post(chat_session_append))
        // tools proxy
        .route("/api/tools/:server/:tool", axum::routing::post(call_tool))
        // policy and approvals
        .route("/api/policy/check", axum::routing::post(policy_check))
        .route("/api/approvals", axum::routing::get(list_approvals).post(create_approval))
        .route("/api/approvals/:id/approve", axum::routing::post(approve_approval))
        .route("/api/approvals/:id/deny", axum::routing::post(deny_approval))
        .route("/api/explain/:id", get(explain_action))
        .with_state(state)
}

async fn health(State(state): State<SharedState>) -> impl IntoResponse {
    crate::metrics::inc_api_request("/health");
    Json(Health { status: "ok", version: state.version })
}

async fn ready() -> impl IntoResponse {
    crate::metrics::inc_api_request("/ready");
    // If config loaded and server is running, return 200
    StatusCode::OK
}

async fn metrics() -> impl IntoResponse {
    crate::metrics::inc_api_request("/metrics");
    let body = crate::metrics::gather_prometheus(env!("CARGO_PKG_VERSION"));
    ([("Content-Type", "text/plain; version=0.0.4")], body)
}

async fn ws_upgrade(ws: WebSocketUpgrade, State(_state): State<SharedState>) -> Response {
    ws.on_upgrade(handle_ws)
}

async fn handle_ws(mut socket: WebSocket) {
    // Echo server
    while let Some(Ok(msg)) = socket.recv().await {
        match msg {
            Message::Text(t) => {
                if socket.send(Message::Text(t)).await.is_err() { break; }
            }
            Message::Binary(b) => {
                if socket.send(Message::Binary(b)).await.is_err() { break; }
            }
            Message::Ping(p) => { let _ = socket.send(Message::Pong(p)).await; }
            Message::Pong(_) => {}
            Message::Close(_) => break,
        }
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
}

#[derive(Serialize)]
struct ApiError { message: String }

async fn list_tasks(State(state): State<SharedState>) -> impl IntoResponse {
    if let Some(mem) = state.handles.memory.as_ref() {
        if let Err(_e) = mem.store.append_event(None, "api:list_tasks", None).await { /* ignore */ }
        match mem.store.list_tasks().await {
            Ok(ts) => Json::<Vec<fm::Task>>(ts).into_response(),
            Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError { message: e.to_string() })).into_response(),
        }
    } else {
        Json::<Vec<fm::Task>>(vec![]).into_response()
    }
}

#[derive(serde::Deserialize)]
struct CreateTaskReq { title: String, status: Option<String>, tags: Option<String> }

async fn create_task(State(state): State<SharedState>, Json(req): Json<CreateTaskReq>) -> impl IntoResponse {
    if let Some(mem) = state.handles.memory.as_ref() {
        if let Err(_e) = mem.store.append_event(None, "api:create_task", Some(&serde_json::json!({"title": req.title}))).await { /* ignore */ }
        match mem.store.create_task(&req.title, req.status.as_deref().unwrap_or("open"), req.tags.as_deref()).await {
            Ok(t) => Json::<fm::Task>(t).into_response(),
            Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError { message: e.to_string() })).into_response(),
        }
    } else {
        (StatusCode::SERVICE_UNAVAILABLE, Json(ApiError { message: "memory not initialized".into() })).into_response()
    }
}

async fn system_map(State(state): State<SharedState>) -> impl IntoResponse {
    if let Some(map) = state.handles.system_map.get_map() {
        Json::<SystemMap>(map).into_response()
    } else {
        StatusCode::SERVICE_UNAVAILABLE.into_response()
    }
}

async fn system_map_digest(State(state): State<SharedState>) -> impl IntoResponse {
    #[derive(Serialize)]
    struct Digest { digest: String }
    let d = state.handles.system_map.get_digest();
    Json(Digest { digest: d })
}

async fn system_map_refresh(State(state): State<SharedState>) -> impl IntoResponse {
    match state.handles.system_map.refresh().await {
        Ok(()) => StatusCode::OK.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError { message: e.to_string() })).into_response(),
    }
}

#[derive(serde::Deserialize)]
struct PackReq { task_id: Option<i64>, token_budget: Option<usize>, k_cards: Option<i64>, expansions: Option<Vec<String>> }

async fn context_pack(State(state): State<SharedState>, Json(req): Json<PackReq>) -> impl IntoResponse {
    let d = state.handles.system_map.get_digest();
    let budget = req.token_budget.unwrap_or(2048);
    let k_cards = req.k_cards.unwrap_or(12);

    #[derive(serde::Serialize)]
    struct EmptyPack { system_digest: String, task_digest: Option<String>, cards: Vec<serde_json::Value>, expansions: Vec<String>, dropped: serde_json::Value }

    if let Some(mem) = state.handles.memory.as_ref() {
        let cards_raw = mem
            .store
            .list_cards(req.task_id, k_cards)
            .await
            .unwrap_or_default();
        // Map to pack cards
        let cards: Vec<crate::memory::context_pack::Card> = cards_raw
            .into_iter()
            .map(|a| {
                let text = a.text;
                let tokens_est = if a.tokens_est > 0 { a.tokens_est as usize } else { estimate_tokens_len(&text) };
                crate::memory::context_pack::Card {
                    atom_id: a.id,
                    text,
                    tokens_est,
                    importance: a.importance as i32,
                    pinned: a.pinned,
                }
            })
            .collect();
        // Prefer pinned/important first (already sorted by SQL), but keep a stable order
        let expansions = req.expansions.unwrap_or_default();
        let pack = crate::memory::context_pack::build_pack(&d, None, cards, budget, expansions);
        return Json(pack).into_response();
    }
    Json(EmptyPack { system_digest: d, task_digest: None, cards: vec![], expansions: vec![], dropped: serde_json::json!({"cards":0,"expansions":0}) }).into_response()
}

fn estimate_tokens_len(text: &str) -> usize { (text.len() / 4).max(1) }

#[derive(serde::Deserialize)]
struct ToolParams { params: serde_json::Value }

async fn call_tool(
    State(state): State<SharedState>,
    AxPath((server, tool)): AxPath<(String, String)>,
    Json(ToolParams { params }): Json<ToolParams>,
) -> impl IntoResponse {
    // Installer apply gating: if missing approval, interrupt with ephemeral prompt
    if server == "installer" && tool == "apply_install" {
        let plan_id = params.get("plan_id").and_then(|v| v.as_str());
        let approval_id = params.get("approval_id").and_then(|v| v.as_str());
        let approve_token = params.get("approve_token").and_then(|v| v.as_str());
        if plan_id.is_none() || approval_id.is_none() || approve_token.is_none() {
            // Create a one-off prompt for TUI
            let cmd_list = plan_id.and_then(|pid| crate::tools::installer_plan_commands(pid)).unwrap_or_default();
            let title = format!("Install plan requires approval: {}", plan_id.unwrap_or("(unknown)"));
            let action = ProposedAction { command: "installer.apply".into(), writes: true, paths: vec![], intent: Some("package install".into()) };
            let prompt = crate::app::EphemeralApproval { id: uuid::Uuid::new_v4().to_string(), title, action, details: serde_json::json!({"commands": cmd_list}) };
            *state.handles.approval_prompt.write() = Some(prompt);
            return (StatusCode::CONFLICT, Json(ApiError { message: "approval required".into() })).into_response();
        }
        let ok = state.handles.approvals.validate_token(approval_id.unwrap(), approve_token.unwrap());
        if !ok {
            return (StatusCode::FORBIDDEN, Json(ApiError { message: "invalid approval token".into() })).into_response();
        }
        if let Some(mem) = state.handles.memory.as_ref() {
            let _ = mem.store.append_event(None, "installer:apply", Some(&serde_json::json!({"plan_id": plan_id}))).await;
        }
    }

    // Log generic MCP call event
    if let Some(mem) = state.handles.memory.as_ref() {
        let _ = mem
            .store
            .append_event(
                None,
                "mcp:call",
                Some(&serde_json::json!({"server": server, "tool": tool})),
            )
            .await;
    }

    match state.handles.tools.invoke(&server, &tool, params).await {
        Ok(v) => Json::<serde_json::Value>(v).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, Json(ApiError { message: e.to_string() })).into_response(),
    }
}

async fn policy_check(State(state): State<SharedState>, axum::Json(action): axum::Json<ProposedAction>) -> impl IntoResponse {
    let decision = state.handles.policy.evaluate(&action);
    Json(decision)
}

async fn create_approval(State(state): State<SharedState>, axum::Json(action): axum::Json<ProposedAction>) -> impl IntoResponse {
    let approval = state.handles.approvals.create(action);
    Json(approval)
}

async fn list_approvals(State(state): State<SharedState>) -> impl IntoResponse {
    Json(state.handles.approvals.list())
}

async fn approve_approval(State(state): State<SharedState>, Path(id): Path<String>) -> Response {
    match state.handles.approvals.approve(&id) {
        Some(a) => Json::<Approval>(a).into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

async fn deny_approval(State(state): State<SharedState>, Path(id): Path<String>) -> Response {
    match state.handles.approvals.deny(&id) {
        Some(a) => Json::<Approval>(a).into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

async fn explain_action(State(state): State<SharedState>, Path(id): Path<String>) -> impl IntoResponse {
    if let Some(a) = state.handles.approvals.get(&id) {
        let card = state.handles.provenance.explain(&id, &a.action);
        Json(card).into_response()
    } else {
        StatusCode::NOT_FOUND.into_response()
    }
}

async fn list_schedules(State(state): State<SharedState>) -> impl IntoResponse {
    let snapshot = state.handles.scheduler.snapshot().await;
    Json(snapshot)
}

async fn list_tool_status(State(state): State<SharedState>) -> impl IntoResponse {
    let v = state.handles.tools.statuses().await;
    Json(v)
}

async fn list_games() -> impl IntoResponse {
    let v = scan_roms_root("/home/kil/games/roms");
    Json(v)
}

async fn run_schedule_job(State(state): State<SharedState>, AxPath((job,)): AxPath<(String,)>) -> Response {
    if let Err(e) = state.handles.scheduler.run_now(&job).await {
        return (StatusCode::BAD_REQUEST, Json(ApiError { message: e.to_string() })).into_response();
    }
    StatusCode::OK.into_response()
}

async fn list_tools(State(state): State<SharedState>) -> impl IntoResponse {
    Json(state.handles.tools.list())
}

async fn voice_test() -> impl IntoResponse {
    #[derive(Serialize)]
    struct VoiceResp { ok: bool, status: u16, note: String }
    #[cfg(feature = "realtime-audio")]
    {
        let info = crate::realtime_audio::devices_info_json();
        return Json(VoiceResp { ok: true, status: 200, note: info.to_string() });
    }
    #[cfg(not(feature = "realtime-audio"))]
    {
        return Json(VoiceResp { ok: false, status: 501, note: "built without realtime-audio feature".into() });
    }
}

#[cfg(feature = "realtime-audio")]
async fn audio_devices() -> impl IntoResponse {
    Json(crate::realtime_audio::devices_info_json())
}
#[cfg(not(feature = "realtime-audio"))]
async fn audio_devices() -> impl IntoResponse { (StatusCode::NOT_IMPLEMENTED, Json(serde_json::json!({"error":"audio disabled"}))).into_response() }

#[derive(Deserialize)]
struct AudioDiagReq {
    #[serde(default = "def_secs")] seconds: u32,
    #[serde(default = "def_sr")] in_sr: u32,
    #[serde(default = "def_chunk")] chunk_ms: u32,
    #[serde(default = "def_sens")] sensitivity: f32,
    #[serde(default = "def_min_speech")] min_speech_ms: u32,
    #[serde(default)] transcribe: Option<bool>,
}
fn def_secs() -> u32 { 6 }
fn def_sr() -> u32 { 16000 }
fn def_chunk() -> u32 { 30 }
fn def_sens() -> f32 { 0.5 }
fn def_min_speech() -> u32 { 300 }

// (audio_diagnose implemented inline in build_router via closure)

#[derive(Deserialize)]
struct BeepReq { #[serde(default = "def_secs_short")] seconds: u32, #[serde(default = "def_out_sr")] out_sr: u32, #[serde(default = "def_freq")] freq_hz: f32 }
fn def_secs_short() -> u32 { 1 }
fn def_out_sr() -> u32 { 48000 }
fn def_freq() -> f32 { 440.0 }

#[cfg(feature = "realtime-audio")]
async fn audio_beep(Json(req): Json<BeepReq>) -> impl IntoResponse {
    let res = tokio::task::spawn_blocking(move || crate::realtime_audio::play_beep(req.seconds, req.out_sr, req.freq_hz)).await;
    match res { Ok(Ok(())) => StatusCode::OK.into_response(), Ok(Err(e)) => (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error":e.to_string()}))).into_response(), Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": format!("join: {}", e)}))).into_response() }
}
#[cfg(not(feature = "realtime-audio"))]
async fn audio_beep(Json(_): Json<BeepReq>) -> impl IntoResponse { (StatusCode::NOT_IMPLEMENTED, Json(serde_json::json!({"error":"audio disabled"}))).into_response() }

#[derive(Deserialize)]
    struct RtStartReq {
        #[serde(default)]
        model: Option<String>,
        #[serde(default)]
        voice: Option<String>,
        #[serde(default)]
        audio: Option<crate::realtime::RealtimeAudioOpts>,
        #[serde(default)]
        instructions: Option<String>,
        #[serde(default)]
        endpoint: Option<String>,
        #[serde(default)]
        transport: Option<String>,
    }

async fn realtime_start(State(state): State<SharedState>, Json(req): Json<RtStartReq>) -> impl IntoResponse {
    crate::metrics::inc_api_request("/api/realtime/start");
    // Fill defaults from config.voice if not provided
    let cfg = state.config.read().clone();
    let (def_model, def_voice, def_endpoint) = cfg.voice.as_ref().map(|v| (
        v.realtime_model.clone(), v.realtime_voice.clone(), v.realtime_endpoint.clone()
    )).unwrap_or((None, None, None));
    let opts = crate::realtime::RealtimeOptions {
        model: req.model.or(def_model),
        voice: req.voice.or(def_voice),
        audio: req.audio,
        instructions: req.instructions,
        endpoint: req.endpoint.or(def_endpoint),
        transport: req.transport,
    };
    match state.handles.realtime.start(opts).await {
        Ok(()) => StatusCode::OK.into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, Json(ApiError { message: e.to_string() })).into_response(),
    }
}

async fn realtime_stop(axum::extract::State(state): axum::extract::State<SharedState>) -> StatusCode {
    crate::metrics::inc_api_request("/api/realtime/stop");
    match state.handles.realtime.stop().await { Ok(()) => StatusCode::OK, Err(_e) => StatusCode::INTERNAL_SERVER_ERROR }
}

async fn realtime_status(State(state): State<SharedState>) -> impl IntoResponse {
    crate::metrics::inc_api_request("/api/realtime/status");
    Json(state.handles.realtime.status())
}

async fn wake_status(State(state): State<SharedState>) -> impl IntoResponse {
    let (active, opts) = state.handles.wake.status();
    Json(serde_json::json!({
        "active": active,
        "phrase": opts.phrase,
        "enabled": opts.enabled,
        "vad_sensitivity": opts.vad_sensitivity,
        "min_speech_ms": opts.min_speech_ms,
        "refractory_ms": opts.refractory_ms,
    }))
}

async fn wake_enable(State(state): State<SharedState>) -> impl IntoResponse {
    state.handles.wake.set_enabled(true);
    #[cfg(feature = "realtime-audio")]
    {
        let w = state.handles.wake.clone();
        let rt = state.handles.realtime.clone();
        tokio::spawn(async move { w.start_task(rt).await; });
    }
    StatusCode::OK
}

async fn wake_disable(State(state): State<SharedState>) -> impl IntoResponse {
    state.handles.wake.set_enabled(false);
    state.handles.wake.stop_task().await;
    StatusCode::OK
}

async fn get_approval_prompt(State(state): State<SharedState>) -> impl IntoResponse {
    if let Some(p) = state.handles.approval_prompt.read().clone() {
        Json(p).into_response()
    } else {
        StatusCode::NO_CONTENT.into_response()
    }
}

#[derive(serde::Deserialize)]
struct ApprovalAnswer { id: String, answer: String }

async fn answer_approval(State(state): State<SharedState>, Json(ans): Json<ApprovalAnswer>) -> impl IntoResponse {
    let mut g = state.handles.approval_prompt.write();
    if let Some(p) = g.clone() {
        if p.id == ans.id {
            // Clear prompt; agent/UI can retry the action as needed
            *g = None;
            return StatusCode::OK.into_response();
        }
    }
    StatusCode::NOT_FOUND.into_response()
}

async fn explain_ephemeral(State(state): State<SharedState>, AxPath((id,)): AxPath<(String,)>) -> impl IntoResponse {
    if let Some(p) = state.handles.approval_prompt.read().clone() {
        if p.id == id {
            let card = state.handles.provenance.explain(&id, &p.action);
            return Json(card).into_response();
        }
    }
    StatusCode::NOT_FOUND.into_response()
}

// ---- Codex API (new/continue) ----
#[derive(serde::Deserialize)]
struct CodexNewReq { prompt: String, repo: Option<String>, config: Option<serde_json::Value> }

#[derive(serde::Serialize)]
struct CodexNewResp { session_id: Option<String>, result: serde_json::Value }

async fn codex_new(State(state): State<SharedState>, Json(req): Json<CodexNewReq>) -> impl IntoResponse {
    let params = serde_json::json!({
        "prompt": req.prompt,
        "repo": req.repo,
        "config": req.config.unwrap_or_else(|| serde_json::json!({}))
    });
    match state.handles.tools.invoke("codex", "new", params).await {
        Ok(result) => {
            let session_id = result.get("session_id").and_then(|v| v.as_str()).map(|s| s.to_string());
            if let (Some(mem), Some(sid)) = (state.handles.memory.as_ref(), session_id.clone()) {
                let _ = mem.store.append_event(None, "mcp_codex_session_configured", Some(&serde_json::json!({
                    "session_id": sid,
                    "source": "api.codex.new",
                    "raw": result
                })) ).await;
                // Record the user's initial prompt for the conversation view
                let _ = mem.store.append_event(None, "mcp_codex_prompt", Some(&serde_json::json!({
                    "session_id": sid,
                    "text": req.prompt,
                })) ).await;
            }
            Json(CodexNewResp { session_id, result }).into_response()
        }
        Err(e) => (StatusCode::BAD_GATEWAY, Json(ApiError { message: e.to_string() })).into_response(),
    }
}

#[derive(serde::Deserialize)]
struct CodexContReq {
    #[serde(alias = "sessionId")]
    session_id: String,
    prompt: Option<String>,
    config: Option<serde_json::Value>,
}

#[derive(serde::Serialize)]
struct CodexContResp { result: serde_json::Value }

async fn codex_continue(State(state): State<SharedState>, Json(req): Json<CodexContReq>) -> impl IntoResponse {
    let params = serde_json::json!({
        "session_id": req.session_id,
        "prompt": req.prompt,
        "config": req.config.unwrap_or_else(|| serde_json::json!({}))
    });
    // Record the user's prompt immediately for the conversation view
    if let (Some(mem), Some(prompt_text)) = (state.handles.memory.as_ref(), req.prompt.clone()) {
        let _ = mem.store.append_event(None, "mcp_codex_prompt", Some(&serde_json::json!({
            "session_id": req.session_id,
            "text": prompt_text,
        })) ).await;
    }
    match state.handles.tools.invoke("codex", "continue", params).await {
        Ok(result) => {
            if let Some(mem) = state.handles.memory.as_ref() {
                let sid = result.get("session_id").and_then(|v| v.as_str()).map(|s| s.to_string()).unwrap_or_else(|| req.session_id.clone());
                let payload = serde_json::json!({ "session_id": sid, "result": result });
                let _ = mem.store.append_event(None, "mcp_codex_continue", Some(&payload)).await;
            }
            Json(CodexContResp { result }).into_response()
        }
        Err(e) => (StatusCode::BAD_GATEWAY, Json(ApiError { message: e.to_string() })).into_response(),
    }
}

// Sessions listing for TUI
#[derive(serde::Serialize)]
struct CodexSessionRow { session_id: String, created_at: String }

async fn codex_sessions(State(state): State<SharedState>) -> impl IntoResponse {
    if let Some(mem) = state.handles.memory.as_ref() {
        let mut out: Vec<CodexSessionRow> = vec![];
        if let Ok(evts) = mem.store.get_recent_events(1000).await {
            for e in evts.into_iter() {
                if e.kind == "mcp_codex_session_configured" {
                    if let Some(payload) = e.payload_json.as_ref().and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok()) {
                        if let Some(sid) = payload.get("session_id").and_then(|v| v.as_str()) {
                            out.push(CodexSessionRow { session_id: sid.to_string(), created_at: e.created_at.to_rfc3339() });
                        }
                    }
                }
            }
        }
        out.sort_by(|a,b| b.created_at.cmp(&a.created_at));
        return Json(out).into_response();
    }
    Json::<Vec<CodexSessionRow>>(vec![]).into_response()
}

// Session detail: aggregate recent events for a given session id into a simple text
async fn codex_session_detail(State(state): State<SharedState>, AxPath((sid,)): AxPath<(String,)>) -> impl IntoResponse {
    #[derive(serde::Serialize)]
    struct Detail { text: String }
    if let Some(mem) = state.handles.memory.as_ref() {
        if let Ok(evts) = mem.store.get_recent_events(2000).await {
            let mut lines: Vec<String> = vec![];
            for e in evts.into_iter().rev() { // oldest first
                match e.kind.as_str() {
                    "mcp_codex_session_configured" => {
                        if let Some(payload) = e.payload_json.as_ref().and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok()) {
                            if payload.get("session_id").and_then(|v| v.as_str()) == Some(sid.as_str()) {
                                lines.push(format!("[{}] session started: {}", e.created_at.format("%Y-%m-%d %H:%M:%S"), sid));
                            }
                        }
                    }
                    "mcp_codex_prompt" => {
                        if let Some(payload) = e.payload_json.as_ref().and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok()) {
                            if payload.get("session_id").and_then(|v| v.as_str()) == Some(sid.as_str()) {
                                let text = payload.get("text").and_then(|v| v.as_str()).unwrap_or("");
                                lines.push(format!("[{}] you: {}", e.created_at.format("%Y-%m-%d %H:%M:%S"), text));
                            }
                        }
                    }
                    "mcp_codex_continue" => {
                        if let Some(payload) = e.payload_json.as_ref().and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok()) {
                            if payload.get("session_id").and_then(|v| v.as_str()) == Some(sid.as_str()) {
                                let snippet = payload.get("result").map(|v| v.to_string()).unwrap_or_else(|| "{}".into());
                                lines.push(format!("[{}] continue: {}", e.created_at.format("%Y-%m-%d %H:%M:%S"), snippet));
                            }
                        }
                    }
                    _ => {}
                }
            }
            return Json(Detail { text: lines.join("\n\n") }).into_response();
        }
    }
    (StatusCode::NOT_FOUND, Json(ApiError { message: "no data".into() })).into_response()
}

// ---- Chat completion with tools ----
#[derive(serde::Deserialize)]
struct ChatMsg { role: String, content: String }

#[derive(serde::Deserialize)]
struct ChatReq { messages: Vec<ChatMsg>, model: Option<String>, max_steps: Option<usize> }

#[derive(serde::Serialize)]
struct ChatResp { reply: String }

async fn chat_complete(State(state): State<SharedState>, Json(req): Json<ChatReq>) -> impl IntoResponse {
    let model = req.model.unwrap_or_else(|| std::env::var("OPENAI_MODEL").unwrap_or_else(|_| "gpt-5".into()));
    let key = match std::env::var("OPENAI_API_KEY") { Ok(k) => k, Err(_) => return (StatusCode::BAD_REQUEST, Json(ApiError { message: "OPENAI_API_KEY not set".into() })).into_response() };
    let client = HttpClient::new();
    // Inject system prompt with local knowledge and tool safety rules
    let mut messages: Vec<serde_json::Value> = vec![serde_json::json!({"role": "system", "content": system_prompt()})];
    messages.extend(req.messages.into_iter().map(|m| serde_json::json!({"role": m.role, "content": m.content})));
    let tools = build_tool_defs(&state.handles.tools);
    let max_steps = req.max_steps.unwrap_or(6);
    for _ in 0..max_steps {
        match openai_chat_once(&client, &key, &model, &messages, &tools).await {
            Ok(OnceResult::Final(reply)) => return Json(ChatResp { reply }).into_response(),
            Ok(OnceResult::ToolCalls(calls, assistant_msg)) => {
                messages.push(assistant_msg);
                for c in calls {
                    let (server, tool) = match c.name.split_once('_') { Some((s, t)) => (s.to_string(), t.to_string()), None => (c.name.clone(), String::new()) };
                    let params = c.arguments.unwrap_or_else(|| serde_json::json!({}));
                    let result = match state.handles.tools.invoke(&server, &tool, params).await { Ok(v) => v, Err(e) => serde_json::json!({"error": e.to_string()}) };
                    messages.push(serde_json::json!({"role": "tool", "tool_call_id": c.id, "content": serde_json::to_string(&result).unwrap_or_else(|_| "{}".into()) }));
                }
                continue;
            }
            Err(e) => {
                tracing::warn!(error=%e, "chat_complete: OpenAI/tool orchestration failed");
                return (StatusCode::BAD_GATEWAY, Json(ApiError { message: e.to_string() })).into_response();
            }
        }
    }
    (StatusCode::BAD_GATEWAY, Json(ApiError { message: "tool loop exceeded".into() })).into_response()
}

fn load_steamgames_user_list() -> Vec<(String, String)> {
    let path = std::path::Path::new("config/steamgames.toml");
    if !path.exists() { return vec![]; }
    let Ok(text) = std::fs::read_to_string(path) else { return vec![] };
    let Ok(val) = toml::from_str::<toml::Value>(&text) else { return vec![] };
    let mut out: Vec<(String,String)> = vec![];
    // Expect a [games] table with key -> appid (string or integer)
    if let Some(tbl) = val.get("games").and_then(|v| v.as_table()) {
        for (name, v) in tbl.iter() {
            let appid = if let Some(s) = v.as_str() { s.to_string() } else if let Some(n) = v.as_integer() { n.to_string() } else { continue };
            out.push((name.to_string(), appid));
        }
    }
    out.sort_by(|a,b| a.0.to_lowercase().cmp(&b.0.to_lowercase()));
    out
}

fn system_prompt() -> String { crate::prompt::base_system_prompt() }

fn scan_roms_root(root: &str) -> Vec<(String, Vec<String>)> {
    use std::fs;
    use std::path::Path;
    let mut out: Vec<(String, Vec<String>)> = vec![];
    let root_path = Path::new(root);
    if !root_path.exists() { return out; }
    let Ok(entries) = fs::read_dir(root_path) else { return out; };
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(ft) = entry.file_type() else { continue; };
        if ft.is_dir() {
            let console = entry.file_name().to_string_lossy().to_string();
            let mut titles: Vec<String> = vec![];
            if let Ok(mut games) = fs::read_dir(&path) {
                while let Some(Ok(g)) = games.next() {
                    let gp = g.path();
                    if gp.is_file() {
                        if let Some(name) = gp.file_stem().and_then(|s| s.to_str()) {
                            let title = name.replace('_', " ");
                            titles.push(title);
                        }
                    }
                }
            }
            if !titles.is_empty() {
                titles.sort();
                out.push((console, titles));
            }
        }
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

fn build_tool_defs(tm: &crate::tools::ToolsManager) -> Vec<serde_json::Value> {
    let mut out = vec![];
    for (server, tools) in tm.list().into_iter() {
        for t in tools {
            let name = format!("{}_{}", server, t).replace('-', "_");
            let (desc, params): (String, serde_json::Value) = if server == "shell" && t == "exec" {
                (
                    "Execute a desktop command with strict policy. Usage: {\"cmd\":\"mgba-qt\",\"args\":[\"/home/kil/games/roms/<console>/<file>\"]} (GB/GBA). For Nintendo DS: {\"cmd\":\"/home/kil/games/emulators/melonDS-x86_64.AppImage\",\"args\":[]}. Optional: {\"wait\": false} to spawn and return a PID.".to_string(),
                    serde_json::json!({
                        "type": "object",
                        "properties": {
                            "cmd": {"type": "string", "description": "Program to run. 'mgba-qt' or the absolute DS emulator path."},
                            "args": {"type": "array", "items": {"type": "string"}, "description": "Arguments list. For mgba-qt, exactly one ROM path under /home/kil/games/roms. For DS emulator, empty."},
                            "wait": {"type": "boolean", "description": "If false, spawn and return pid; defaults to true (wait for exit)."}
                        },
                        "required": ["cmd"],
                        "additionalProperties": false
                    })
                )
            } else if server == "shell" && t == "which" {
                (
                    "Resolve a command name to a full path (PATH lookup).".to_string(),
                    serde_json::json!({
                        "type": "object",
                        "properties": {"cmd": {"type": "string", "description": "Command name, e.g., 'mgba-qt'"}},
                        "required": ["cmd"],
                        "additionalProperties": false
                    })
                )
            } else if server == "shell" && (t == "list_dir" || t == "read_file") {
                let d = if t == "list_dir" { "List entries in a directory." } else { "Read a UTF-8 text file." };
                (
                    d.to_string(),
                    serde_json::json!({
                        "type": "object",
                        "properties": {"path": {"type": "string", "description": "Filesystem path"}},
                        "required": ["path"],
                        "additionalProperties": false
                    })
                )
            } else if server == "steam" && t == "installed" {
                (
                    "List installed Steam games by reading local Steam app manifests (~/.local/share/Steam/steamapps).".to_string(),
                    serde_json::json!({
                        "type": "object",
                        "properties": {
                            "root": {"type": "string", "description": "Optional override for steamapps directory"}
                        },
                        "additionalProperties": false
                    })
                )
            } else if server == "steam" && t == "launch" {
                (
                    "Launch a Steam game (steam -applaunch APPID).".to_string(),
                    serde_json::json!({
                        "type": "object",
                        "properties": {"appid": {"type": "string", "description": "Steam AppID (digits)"}},
                        "required": ["appid"],
                        "additionalProperties": false
                    })
                )
            } else {
                (
                    format!("Call {}.{} via Foreman MCP", server, t),
                    serde_json::json!({"type": "object", "properties": {}, "additionalProperties": true})
                )
            };
            out.push(serde_json::json!({
                "type": "function",
                "function": {
                    "name": name,
                    "description": desc,
                    "parameters": params
                }
            }));
        }
    }
    out
}

enum OnceResult { Final(String), ToolCalls(Vec<ToolCall>, serde_json::Value) }

struct ToolCall { id: String, name: String, arguments: Option<serde_json::Value> }

async fn openai_chat_once(client: &HttpClient, key: &str, model: &str, messages: &Vec<serde_json::Value>, tools: &Vec<serde_json::Value>) -> anyhow::Result<OnceResult> {
    #[derive(serde::Deserialize)]
    struct Choice { message: serde_json::Value }
    #[derive(serde::Deserialize)]
    struct Resp { choices: Vec<Choice> }
    let body = serde_json::json!({"model": model, "messages": messages, "tools": tools, "tool_choice": "auto"});
    let resp = client.post("https://api.openai.com/v1/chat/completions").bearer_auth(key).header("content-type","application/json").json(&body).send().await?;
    if !resp.status().is_success() {
        let status = resp.status();
        let txt = resp.text().await.unwrap_or_default();
        let snip = if txt.len() > 400 { format!("{}…", &txt[..400]) } else { txt };
        anyhow::bail!(format!("openai http {}: {}", status, snip));
    }
    let v: Resp = resp.json().await?;
    let msg = v.choices.get(0).map(|c| c.message.clone()).unwrap_or(serde_json::json!({"role":"assistant","content":""}));
    if let Some(tc) = msg.get("tool_calls").and_then(|x| x.as_array()) {
        let mut calls = vec![];
        for c in tc {
            let id = c.get("id").and_then(|s| s.as_str()).unwrap_or("").to_string();
            let name = c.get("function").and_then(|f| f.get("name")).and_then(|s| s.as_str()).unwrap_or("").to_string();
            let args = c.get("function").and_then(|f| f.get("arguments")).and_then(|s| s.as_str()).and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok());
            calls.push(ToolCall { id, name, arguments: args });
        }
        return Ok(OnceResult::ToolCalls(calls, msg));
    }
    let reply = msg.get("content").and_then(|s| s.as_str()).unwrap_or("").to_string();
    Ok(OnceResult::Final(reply))
}

#[derive(serde::Deserialize)]
struct ChatStreamReq { messages: Vec<ChatMsg>, model: Option<String>, max_steps: Option<usize> }

async fn chat_stream(State(state): State<SharedState>, Json(req): Json<ChatStreamReq>) -> impl IntoResponse {
    let model = req.model.unwrap_or_else(|| std::env::var("OPENAI_MODEL").unwrap_or_else(|_| "gpt-5".into()));
    let key = match std::env::var("OPENAI_API_KEY") { Ok(k) => k, Err(_) => return (StatusCode::BAD_REQUEST, "OPENAI_API_KEY not set").into_response() };
    let client = HttpClient::new();
    let tools = build_tool_defs(&state.handles.tools);
    let max_steps = req.max_steps.unwrap_or(6);

    let (tx, rx) = tokio::sync::mpsc::channel::<String>(16);
    let mut messages: Vec<serde_json::Value> = vec![serde_json::json!({"role": "system", "content": system_prompt()})];
    messages.extend(req.messages.into_iter().map(|m| serde_json::json!({"role": m.role, "content": m.content})));
    tokio::spawn(async move {
        for _ in 0..max_steps {
            match openai_chat_once(&client, &key, &model, &messages, &tools).await {
                Ok(OnceResult::ToolCalls(calls, assistant_msg)) => {
                    let _ = tx.send(format!("event: tool_calls\n")).await;
                    for c in calls.iter() {
                        let name = &c.name;
                        let args = c.arguments.clone().unwrap_or_else(|| serde_json::json!({}));
                        let _ = tx.send(format!("event: tool_call\n")).await;
                        let _ = tx.send(format!("data: {}\n\n", serde_json::json!({"name": name, "arguments": args}))).await;
                        let (server, tool) = match name.split_once('_') { Some((s,t)) => (s.to_string(), t.to_string()), None => (name.clone(), String::new()) };
                        let result = match state.handles.tools.invoke(&server, &tool, args).await { Ok(v) => v, Err(e) => serde_json::json!({"error": e.to_string()}) };
                        let snip = serde_json::to_string(&result).unwrap_or_else(|_| "{}".into());
                        let _ = tx.send("event: tool_result\n".to_string()).await;
                        let _ = tx.send(format!("data: {}\n\n", serde_json::json!({"name": name, "result": snip}))).await;
                        // Append tool message
                        messages.push(serde_json::json!({"role":"tool","tool_call_id": c.id, "content": snip}));
                    }
                    // Add assistant tool_calls message so model sees its own tool calls
                    messages.push(assistant_msg);
                    continue;
                }
                Ok(OnceResult::Final(_)) => {
                    // Stream final tokens with OpenAI stream=true with no tools (to avoid further calls)
                    if let Err(e) = openai_stream_tokens(&client, &key, &model, &messages, tx.clone()).await {
                        let _ = tx.send(format!("event: error\n")).await;
                        let _ = tx.send(format!("data: {}\n\n", serde_json::json!({"message": e.to_string()}))).await;
                    }
                    break;
                }
                Err(e) => {
                    let _ = tx.send(format!("event: error\n")).await;
                    let _ = tx.send(format!("data: {}\n\n", serde_json::json!({"message": e.to_string()}))).await;
                    break;
                }
            }
        }
        let _ = tx.send("event: done\n\n".to_string()).await;
    });

    let stream = tokio_stream::wrappers::ReceiverStream::new(rx).map(|line| Ok::<_, std::convert::Infallible>(axum::body::Bytes::from(line)));
    let body = axum::body::Body::from_stream(stream);
    axum::response::Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/event-stream")
        .header("Cache-Control", "no-cache")
        .header("Connection", "keep-alive")
        .body(body)
        .unwrap()
}

async fn openai_stream_tokens(client: &HttpClient, key: &str, model: &str, messages: &Vec<serde_json::Value>, tx: tokio::sync::mpsc::Sender<String>) -> anyhow::Result<()> {
    let body = serde_json::json!({"model": model, "messages": messages, "stream": true});
    let mut resp = client.post("https://api.openai.com/v1/chat/completions").bearer_auth(key).header("content-type","application/json").json(&body).send().await?;
    if !resp.status().is_success() { anyhow::bail!(format!("openai http {}", resp.status())); }
    let mut buf: Vec<u8> = vec![];
    let mut s = resp.bytes_stream();
    while let Some(chunk) = s.next().await {
        let bytes = chunk?;
        buf.extend_from_slice(&bytes);
        loop {
            if let Some(pos) = buf.iter().position(|&b| b == b'\n') {
                let line = buf.drain(..=pos).collect::<Vec<u8>>();
                let text = String::from_utf8_lossy(&line).to_string();
                if let Some(rest) = text.strip_prefix("data: ") {
                    let data = rest.trim();
                    if data == "[DONE]" { return Ok(()); }
                    if !data.is_empty() {
                        if let Ok(v) = serde_json::from_str::<serde_json::Value>(data) {
                            if let Some(choice) = v.get("choices").and_then(|c| c.as_array()).and_then(|a| a.get(0)) {
                                if let Some(delta) = choice.get("delta") {
                                    if let Some(piece) = delta.get("content").and_then(|c| c.as_str()) {
                                        let _ = tx.send("event: token\n".to_string()).await;
                                        let _ = tx.send(format!("data: {}\n\n", piece)).await;
                                    }
                                }
                            }
                        }
                    }
                }
            } else { break; }
        }
    }
    Ok(())
}

// ---- Chat session persistence ----
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
struct ChatMessage { role: String, content: String, at: Option<String> }

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
struct ChatSession { id: String, messages: Vec<ChatMessage> }

#[derive(serde::Serialize, Clone, Debug)]
struct SessionInfo { id: String, updated_at: String, title: Option<String> }

fn chat_dir(state: &SharedState) -> std::path::PathBuf {
    let base = state.handles.system_map.map_path().parent().unwrap_or(std::path::Path::new("."));
    base.join("chats")
}

async fn read_session(path: &std::path::Path) -> anyhow::Result<ChatSession> {
    let bytes = tokio::fs::read(path).await?;
    Ok(serde_json::from_slice(&bytes)?)
}

async fn write_session(path: &std::path::Path, s: &ChatSession) -> anyhow::Result<()> {
    if let Some(p) = path.parent() { tokio::fs::create_dir_all(p).await.ok(); }
    let bytes = serde_json::to_vec_pretty(s)?;
    tokio::fs::write(path, bytes).await?;
    Ok(())
}

async fn chat_sessions_list(State(state): State<SharedState>) -> impl IntoResponse {
    let dir = chat_dir(&state);
    let mut entries: Vec<SessionInfo> = vec![];
    if let Ok(mut rd) = tokio::fs::read_dir(&dir).await {
        while let Ok(Some(e)) = rd.next_entry().await {
            if e.file_type().await.map(|t| t.is_file()).unwrap_or(false) {
                let path = e.path();
                let id = path.file_stem().and_then(|s| s.to_str()).unwrap_or("").to_string();
                let meta = e.metadata().await.ok();
                let mtime = meta.and_then(|m| m.modified().ok()).unwrap_or(std::time::SystemTime::UNIX_EPOCH);
                let updated_at = chrono::DateTime::<chrono::Utc>::from(mtime).to_rfc3339();
                let title = read_session(&path).await.ok().and_then(|s| s.messages.iter().find(|m| m.role=="user").map(|m| m.content.clone()));
                entries.push(SessionInfo { id, updated_at, title });
            }
        }
    }
    entries.sort_by(|a,b| b.updated_at.cmp(&a.updated_at));
    Json(entries)
}

async fn chat_sessions_latest(State(state): State<SharedState>) -> impl IntoResponse {
    let dir = chat_dir(&state);
    if let Ok(mut rd) = tokio::fs::read_dir(&dir).await {
        let mut best: Option<(std::time::SystemTime, std::path::PathBuf)> = None;
        while let Ok(Some(e)) = rd.next_entry().await {
            if e.file_type().await.map(|t| t.is_file()).unwrap_or(false) {
                if let Ok(meta) = e.metadata().await {
                    if let Ok(mtime) = meta.modified() {
                        if best.as_ref().map(|(t,_)| mtime > *t).unwrap_or(true) {
                            best = Some((mtime, e.path()));
                        }
                    }
                }
            }
        }
        if let Some((_, path)) = best {
            match read_session(&path).await {
                Ok(s) => return Json(s).into_response(),
                Err(_) => {}
            }
        }
    }
    StatusCode::NO_CONTENT.into_response()
}

async fn chat_sessions_create(State(state): State<SharedState>) -> impl IntoResponse {
    let id = uuid::Uuid::new_v4().to_string();
    let s = ChatSession { id: id.clone(), messages: vec![] };
    let path = chat_dir(&state).join(format!("{}.json", id));
    if let Err(e) = write_session(&path, &s).await {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError { message: e.to_string() })).into_response();
    }
    Json(s).into_response()
}

async fn chat_session_get(State(state): State<SharedState>, Path(id): Path<String>) -> impl IntoResponse {
    let path = chat_dir(&state).join(format!("{}.json", id));
    match read_session(&path).await {
        Ok(s) => Json(s).into_response(),
        Err(_) => StatusCode::NOT_FOUND.into_response(),
    }
}

#[derive(serde::Deserialize)]
struct AppendReq { role: String, content: String }
async fn chat_session_append(State(state): State<SharedState>, Path(id): Path<String>, Json(req): Json<AppendReq>) -> impl IntoResponse {
    let path = chat_dir(&state).join(format!("{}.json", id));
    let mut s = match read_session(&path).await { Ok(s) => s, Err(_) => ChatSession { id: id.clone(), messages: vec![] } };
    let at = chrono::Utc::now().to_rfc3339();
    s.messages.push(ChatMessage { role: req.role, content: req.content, at: Some(at) });
    match write_session(&path, &s).await {
        Ok(()) => StatusCode::OK.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError { message: e.to_string() })).into_response(),
    }
}

async fn chat_session_delete(State(state): State<SharedState>, Path(id): Path<String>) -> impl IntoResponse {
    let path = chat_dir(&state).join(format!("{}.json", id));
    match tokio::fs::remove_file(&path).await {
        Ok(()) => StatusCode::OK.into_response(),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => StatusCode::NOT_FOUND.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError { message: e.to_string() })).into_response(),
    }
}

// ---- Memory APIs ----
#[derive(serde::Deserialize)]
struct SearchQ { q: String, task_id: Option<i64>, k: Option<i64> }

async fn memory_search(State(state): State<SharedState>, axum::extract::Query(SearchQ { q, task_id, k }): axum::extract::Query<SearchQ>) -> impl IntoResponse {
    if let Some(mem) = state.handles.memory.as_ref() {
        match mem.store.search_atoms(&q, task_id, k.unwrap_or(20)).await {
            Ok(hits) => return Json(hits).into_response(),
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError { message: e.to_string() })).into_response(),
        }
    }
    (StatusCode::SERVICE_UNAVAILABLE, Json(ApiError { message: "memory not initialized".into() })).into_response()
}

async fn get_atom(State(state): State<SharedState>, Path(id): Path<i64>) -> impl IntoResponse {
    if let Some(mem) = state.handles.memory.as_ref() {
        match mem.store.get_atom_full(id).await {
            Ok(Some(a)) => return Json(a).into_response(),
            Ok(None) => return StatusCode::NOT_FOUND.into_response(),
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError { message: e.to_string() })).into_response(),
        }
    }
    (StatusCode::SERVICE_UNAVAILABLE, Json(ApiError { message: "memory not initialized".into() })).into_response()
}

async fn pin_atom(State(state): State<SharedState>, Path(id): Path<i64>) -> impl IntoResponse {
    if let Some(mem) = state.handles.memory.as_ref() {
        match mem.store.pin_atom(id, true).await {
            Ok(()) => return Json(serde_json::json!({"pinned": true})).into_response(),
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError { message: e.to_string() })).into_response(),
        }
    }
    (StatusCode::SERVICE_UNAVAILABLE, Json(ApiError { message: "memory not initialized".into() })).into_response()
}

async fn unpin_atom(State(state): State<SharedState>, Path(id): Path<i64>) -> impl IntoResponse {
    if let Some(mem) = state.handles.memory.as_ref() {
        match mem.store.pin_atom(id, false).await {
            Ok(()) => return Json(serde_json::json!({"pinned": false})).into_response(),
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError { message: e.to_string() })).into_response(),
        }
    }
    (StatusCode::SERVICE_UNAVAILABLE, Json(ApiError { message: "memory not initialized".into() })).into_response()
}

// ---- Context expand ----
#[derive(serde::Deserialize)]
struct ExpandReq { handle: String, depth: Option<usize> }

#[derive(serde::Serialize)]
struct ExpandResp { handle: String, chunk: String, tokens_est: usize, done: bool }

async fn context_expand(State(state): State<SharedState>, Json(req): Json<ExpandReq>) -> impl IntoResponse {
    let handle = req.handle.clone();
    if let Some(mem) = state.handles.memory.as_ref() {
        if let Some(rest) = handle.strip_prefix("expand://task/") {
            if let Ok(task_id) = rest.parse::<i64>() {
                match mem.store.get_atoms_by_task(task_id).await {
                    Ok(mut atoms) => {
                        let n = req.depth.unwrap_or(5);
                        atoms.truncate(n.min(atoms.len()));
                        let chunk = atoms.into_iter().rev().map(|a| a.text).collect::<Vec<_>>().join("\n\n");
                        let tokens_est = estimate_tokens_len(&chunk);
                        return Json(ExpandResp { handle, chunk, tokens_est, done: true }).into_response();
                    }
                    Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError { message: e.to_string() })).into_response(),
                }
            }
        } else if let Some(rest) = handle.strip_prefix("expand://atom/") {
            if let Ok(atom_id) = rest.parse::<i64>() {
                match mem.store.get_atom_full(atom_id).await {
                    Ok(Some(a)) => {
                        let tokens_est = estimate_tokens_len(&a.text);
                        return Json(ExpandResp { handle, chunk: a.text, tokens_est, done: true }).into_response();
                    }
                    Ok(None) => return StatusCode::NOT_FOUND.into_response(),
                    Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError { message: e.to_string() })).into_response(),
                }
            }
        } else if let Some(rest) = handle.strip_prefix("expand://artifact/") {
            // Optional fragment like #0-2000 ignored for now; read head safely
            let id_part = rest.split('#').next().unwrap_or(rest);
            if let Ok(artifact_id) = id_part.parse::<i64>() {
                match mem.store.get_artifact(artifact_id).await {
                    Ok(Some(art)) => {
                        let path = std::path::PathBuf::from(art.path);
                        match tokio::fs::read(&path).await {
                            Ok(bytes) => {
                                let max = 2000usize;
                                let slice = if bytes.len() > max { &bytes[..max] } else { &bytes[..] };
                                let chunk = String::from_utf8_lossy(slice).to_string();
                                let tokens_est = estimate_tokens_len(&chunk);
                                return Json(ExpandResp { handle, chunk, tokens_est, done: bytes.len() <= max }).into_response();
                            }
                            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError { message: e.to_string() })).into_response(),
                        }
                    }
                    Ok(None) => return StatusCode::NOT_FOUND.into_response(),
                    Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError { message: e.to_string() })).into_response(),
                }
            }
        }
    }
    (StatusCode::BAD_REQUEST, Json(ApiError { message: "unsupported handle or memory not available".into() })).into_response()
}
