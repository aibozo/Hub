#![cfg(feature = "tui")]
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use ratatui::{prelude::*, widgets::*};
use std::time::{Instant, Duration};
use unicode_width::{UnicodeWidthStr, UnicodeWidthChar};

use crate::theme;
use crate::screens;
#[cfg(feature = "http")]
use self::net::EphemeralPrompt;
#[cfg(feature = "http")]
use self::net::{CodexSession};

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum Screen { Chat, Dashboard, Tasks, Memory, Tools, Codex, Reports, Research, Settings }

pub struct App {
    pub active: Screen,
    pub input: String,
    pub status: String,
    pub sys_digest: String,
    pub chat_model: String,
    pub mouse_capture: bool,
    pub want_mouse_capture: bool,
    pub tasks: Vec<String>,
    pub approvals: Vec<String>,
    pub tool_output: Vec<String>,
    pub ep_prompt: Option<EphemeralPrompt>,
    pub ep_sel: usize,
    pub ep_explain: Option<String>,
    // Focus management
    pub focus_ix: u8,
    // Chat
    pub chat_messages: Vec<ChatMsg>,
    pub chat_scroll: usize,
    pub chat_session_id: Option<String>,
    pub chat_modal: Option<ChatModal>,
    pub chat_stream: bool,
    pub show_help: bool,
    pub reports: Vec<String>,
    pub report_sel: usize,
    pub report_content: String,
    // Voice (push-to-talk)
    pub voice_ptt: bool,
    #[cfg(feature = "voice")]
    pub voice_rec: Option<crate::audio::VoiceRecorder>,
    pub voice_last_seen: Option<Instant>,
    // Realtime status
    pub rt_active: bool,
    pub last_rt_poll: Option<Instant>,
    // Dashboard
    pub health_ok: bool,
    pub health_version: String,
    pub sched_rows: Vec<String>,
    pub metrics_summary: String,
    // Tools runner
    pub tools_list: Vec<(String, Vec<String>)>,
    pub tool_server_sel: usize,
    pub tool_tool_sel: usize,
    pub tools_status: std::collections::HashMap<String, String>,
    pub tool_params: String,
    pub tool_output_text: String,
    pub editing_params: bool,
    pub toasts: Vec<Toast>,
    pub dash_reports: Vec<String>,
    // Memory screen
    pub mem_query: String,
    pub mem_results: Vec<MemHit>,
    pub mem_sel: usize,
    pub mem_atom: Option<MemAtom>,
    pub mem_pack_summary: String,
    // Codex screen
    pub codex_sessions: Vec<CodexSession>,
    pub codex_sel: usize,
    pub codex_detail: String,
    // Transient Ctrl-hold hints overlay
    pub ctrl_hints_until: Option<Instant>,
    // Project selection
    pub proj_list: Vec<String>,
    pub proj_sel: usize,
    pub proj_current: Option<String>,
    pub proj_modal_open: bool,
    // Steam quick-launch picker
    pub steam_modal_open: bool,
    pub steam_list: Vec<(String, String)>, // (name, appid)
    pub steam_sel: usize,
    // Research screen
    pub research_results: Vec<String>,
    pub research_sel: usize,
    pub research_details: String,
}

impl Default for App {
    fn default() -> Self {
        Self {
            active: Screen::Chat,
            input: String::new(),
            status: "Disconnected".into(),
            sys_digest: String::new(),
            chat_model: "gpt-5".into(),
            mouse_capture: true,
            want_mouse_capture: true,
            tasks: vec![],
            approvals: vec![],
            tool_output: vec![],
            ep_prompt: None,
            ep_sel: 0,
            ep_explain: None,
            // focus/input
            focus_ix: 1,
            // chat
            chat_messages: vec![],
            chat_scroll: 0,
            chat_session_id: None,
            chat_modal: None,
            chat_stream: false,
            show_help: false,
            reports: vec![],
            report_sel: 0,
            report_content: String::new(),
            // voice
            voice_ptt: false,
            #[cfg(feature = "voice")]
            voice_rec: None,
            voice_last_seen: None,
            rt_active: false,
            last_rt_poll: None,
            health_ok: false,
            health_version: String::new(),
            sched_rows: vec![],
            metrics_summary: String::new(),
            tools_list: vec![],
            tool_server_sel: 0,
            tool_tool_sel: 0,
            tools_status: std::collections::HashMap::new(),
            tool_params: "{}".into(),
            tool_output_text: String::new(),
            editing_params: false,
            toasts: vec![],
            dash_reports: vec![],
            mem_query: String::new(),
            mem_results: vec![],
            mem_sel: 0,
            mem_atom: None,
            mem_pack_summary: String::new(),
            codex_sessions: vec![],
            codex_sel: 0,
            codex_detail: String::new(),
            ctrl_hints_until: None,
            proj_list: vec![],
            proj_sel: 0,
            proj_current: None,
            proj_modal_open: false,
            steam_modal_open: false,
            steam_list: vec![],
            steam_sel: 0,
            research_results: vec![],
            research_sel: 0,
            research_details: String::new(),
        }
    }
}

#[derive(Copy, Clone)]
enum ToastKind { Info, Success, Warn, Error }
struct Toast { msg: String, kind: ToastKind, at: Instant }

fn ensure_logs_dir() {
    let _ = std::fs::create_dir_all("storage/logs");
}

fn log_line(level: &str, msg: &str) {
    ensure_logs_dir();
    let path = std::path::Path::new("storage/logs/ui-tui.log");
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
        use std::io::Write as _;
        let ts = match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) { Ok(d) => d.as_secs(), Err(_) => 0 };
        let _ = writeln!(f, "[{}][{}] {}", ts, level, msg.replace('\n', " "));
    }
}

fn push_toast(app: &mut App, msg: impl Into<String>, kind: ToastKind) {
    let m = msg.into();
    app.toasts.push(Toast { msg: m.clone(), kind, at: Instant::now() });
    match kind { ToastKind::Error => log_line("ERROR", &m), ToastKind::Warn => log_line("WARN", &m), ToastKind::Success => log_line("INFO", &m), ToastKind::Info => log_line("INFO", &m) }
    if app.toasts.len() > 50 { app.toasts.drain(0..app.toasts.len()-50); }
}

fn load_steamgames_list() -> Vec<(String, String)> {
    let path = std::path::Path::new("config/steamgames.toml");
    if !path.exists() { return vec![]; }
    let text = match std::fs::read_to_string(path) { Ok(t) => t, Err(_) => return vec![] };
    let val: toml::Value = match toml::from_str(&text) { Ok(v) => v, Err(_) => return vec![] };
    let mut out: Vec<(String, String)> = vec![];
    if let Some(tbl) = val.get("games").and_then(|v| v.as_table()) {
        for (name, v) in tbl.iter() {
            let appid = if let Some(s) = v.as_str() { s.to_string() } else if let Some(n) = v.as_integer() { n.to_string() } else { continue };
            out.push((name.to_string(), appid));
        }
    }
    out.sort_by(|a,b| a.0.to_lowercase().cmp(&b.0.to_lowercase()));
    out
}

pub async fn run() -> anyhow::Result<()> {
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    crossterm::execute!(stdout, crossterm::terminal::EnterAlternateScreen, crossterm::event::EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::default();
    app.status = format!("ui-tui v{}", env!("CARGO_PKG_VERSION"));
    // Mouse capture starts enabled; app state reflects that
    app.mouse_capture = true;
    app.want_mouse_capture = true;

    // Try fetch system digest && tasks when http feature present
    #[cfg(feature = "http")]
    {
        if let Ok(d) = net::get_system_digest().await { app.sys_digest = d; app.status = "Connected".into(); }
        if let Ok(ts) = net::list_tasks().await { app.tasks = ts; }
        if let Ok(aps) = net::list_approvals().await { app.approvals = aps; }
        if let Ok(rs) = net::list_reports().await { app.reports = rs.clone(); app.dash_reports = rs; }
        if let Ok((ok, ver)) = net::health().await { app.health_ok = ok; app.health_version = ver.clone(); if ok { push_toast(&mut app, format!("Connected (v{})", ver), ToastKind::Success); } }
        if let Ok(rows) = net::list_schedules().await { app.sched_rows = rows; }
        if let Ok(mtxt) = net::metrics_text().await { app.metrics_summary = net::summarize_metrics(&mtxt); }
        if let Ok(lst) = net::list_tools().await { app.tools_list = lst; }
        if let Ok(st) = net::list_tool_status().await { app.tools_status = st.into_iter().collect(); }
        if let Ok(Some(sess)) = net::chat_latest().await { app.chat_session_id = Some(sess.id.clone()); app.chat_messages = sess.messages; app.status = format!("Loaded chat {}", &sess.id[..8]); }
        if let Ok(cs) = net::codex_sessions().await { app.codex_sessions = cs; if let Some(s) = app.codex_sessions.get(0) { if let Ok(d) = net::codex_session_detail(&s.session_id).await { app.codex_detail = d; } } }
    }

    // Channel for background events (chat replies/stream)
    let (evt_tx, mut evt_rx) = tokio::sync::mpsc::unbounded_channel::<ChatEvent>();

    let res = loop {
        terminal.draw(|f| ui(f, &app))?;
        // Drain background events and update UI state
        while let Ok(ev) = evt_rx.try_recv() {
            match ev {
                ChatEvent::AssistantReply(reply) => {
                    app.chat_messages.push(ChatMsg { role: "assistant".into(), content: reply });
                    // Do not force-scroll if user scrolled up; staying at bottom is implicit when chat_scroll==0
                }
                ChatEvent::StreamDelta(delta) => {
                    if let Some(last) = app.chat_messages.last_mut() {
                        if last.role == "assistant" { last.content.push_str(&delta); }
                    }
                }
                ChatEvent::ToolNote(line) => {
                    app.chat_messages.push(ChatMsg { role: "assistant".into(), content: line });
                }
                ChatEvent::StreamDone => { push_toast(&mut app, "Chat complete", ToastKind::Success); }
                ChatEvent::Error(msg) => { push_toast(&mut app, msg, ToastKind::Error); }
                ChatEvent::VoiceTranscript(t) => {
                    if !t.is_empty() {
                        if !app.input.is_empty() { app.input.push(' '); }
                        app.input.push_str(&t);
                        push_toast(&mut app, "Voice: transcribed", ToastKind::Success);
                        app.status = "Voice: transcribed".into();
                    }
                }
            }
        }
        // Periodically poll realtime status
        #[cfg(feature = "http")]
        {
            let now = Instant::now();
            let should_poll = app.last_rt_poll.map(|t| now.duration_since(t) > Duration::from_millis(1500)).unwrap_or(true);
            if should_poll {
                match net::realtime_status().await {
                    Ok((active, err)) => {
                        let was_active = app.rt_active;
                        app.rt_active = active;
                        if active {
                            if app.status.starts_with("realtime: starting") { app.status = "realtime: active".into(); push_toast(&mut app, "Realtime: Active", ToastKind::Success); }
                        } else if let Some(e) = err {
                            if app.status.starts_with("realtime: starting") || was_active { app.status = format!("realtime error: {}", e); push_toast(&mut app, format!("Realtime error: {}", e), ToastKind::Error); }
                        }
                        // Refresh chat messages from latest session so voice transcripts show up live
                        if let Ok(Some(sess)) = net::chat_latest().await {
                            let prev_len = app.chat_messages.len();
                            if app.chat_session_id.as_deref() != Some(&sess.id) {
                                app.chat_session_id = Some(sess.id.clone());
                            }
                            let new_len = sess.messages.len();
                            app.chat_messages = sess.messages;
                            // Do not reset scroll unless user is already at bottom and new messages arrived (implicit bottom)
                            if new_len > prev_len {
                                // If user is at bottom (offset==0), keep bottom by leaving chat_scroll as-is (0). If scrolled, preserve.
                            }
                        }
                    }
                    Err(_) => {}
                }
                app.last_rt_poll = Some(now);
            }
        }
        if event::poll(std::time::Duration::from_millis(100))? {
            match event::read()? {
                Event::Key(k) if k.kind == KeyEventKind::Press => {
                    // Project picker modal handling
                    if app.proj_modal_open {
                        match k.code {
                            KeyCode::Esc => { app.proj_modal_open = false; }
                            KeyCode::Up => { if app.proj_sel > 0 { app.proj_sel -= 1; } }
                            KeyCode::Down => { if !app.proj_list.is_empty() { app.proj_sel = (app.proj_sel + 1).min(app.proj_list.len().saturating_sub(1)); } }
                            KeyCode::Enter => {
                                if let Some(p) = app.proj_list.get(app.proj_sel).cloned() {
                                    app.proj_current = Some(p.clone());
                                    app.status = format!("Project set: {}", p);
                                    push_toast(&mut app, format!("Project set: {}", p), ToastKind::Success);
                                    app.proj_modal_open = false;
                                }
                            }
                            _ => {}
                        }
                        continue;
                    }
                    // Steam games picker modal handling
                    if app.steam_modal_open {
                        match k.code {
                            KeyCode::Esc => { app.steam_modal_open = false; }
                            KeyCode::Up => { if app.steam_sel > 0 { app.steam_sel -= 1; } }
                            KeyCode::Down => { if !app.steam_list.is_empty() { app.steam_sel = (app.steam_sel + 1).min(app.steam_list.len().saturating_sub(1)); } }
                            KeyCode::Enter => {
                                if let Some((_, appid)) = app.steam_list.get(app.steam_sel).cloned() {
                                    app.tool_params = format!("{{\"appid\": \"{}\"}}", appid);
                                    app.steam_modal_open = false;
                                    app.status = format!("Steam appid selected: {}", appid);
                                    push_toast(&mut app, format!("Steam: selected {}", appid), ToastKind::Success);
                                }
                            }
                            _ => {}
                        }
                        continue;
                    }
                    // If any Ctrl combo is pressed, briefly show hotkey hints overlay
                    if k.modifiers.contains(KeyModifiers::CONTROL) {
                        app.ctrl_hints_until = Some(Instant::now() + Duration::from_millis(1200));
                    }
                    // Handle chat modal first if open
                    if let Some(ChatModal::Delete { list, sel }) = &mut app.chat_modal {
                        match k.code {
                            KeyCode::Esc => { app.chat_modal = None; continue; }
                            KeyCode::Up => { if *sel > 0 { *sel -= 1; } continue; }
                            KeyCode::Down => { if !list.is_empty() { *sel = (*sel + 1).min(list.len()-1); } continue; }
                            KeyCode::Enter => {
                                #[cfg(feature = "http")]
                                {
                                    if let Some(si) = list.get(*sel).cloned() {
                                        match net::chat_delete(&si.id).await {
                                            Ok(()) => {
                                                push_toast(&mut app, format!("Deleted {}", &si.id[..8]), ToastKind::Success);
                                                if app.chat_session_id.as_deref() == Some(&si.id) { app.chat_session_id = None; app.chat_messages.clear(); }
                                                app.chat_modal = None;
                                            }
                                            Err(e) => { push_toast(&mut app, format!("Delete error: {}", e), ToastKind::Error); }
                                        }
                                    }
                                }
                                continue;
                            }
                            _ => { /* ignore */ continue; }
                        }
                    }
                    if app.ep_prompt.is_some() {
                        match k.code {
                            KeyCode::Left | KeyCode::Up => { if app.ep_sel > 0 { app.ep_sel -= 1; } }
                            KeyCode::Right | KeyCode::Down => { if app.ep_sel < 2 { app.ep_sel += 1; } }
                            KeyCode::Enter => {
                                let id = app.ep_prompt.as_ref().unwrap().id.clone();
                                match app.ep_sel {
                                    0 => { #[cfg(feature = "http")] { let _ = net::answer_approval(&id, "yes").await; } app.ep_prompt=None; app.ep_explain=None; app.status = "Approved".into(); }
                                    1 => { #[cfg(feature = "http")] { let _ = net::answer_approval(&id, "no").await; } app.ep_prompt=None; app.ep_explain=None; app.status = "Denied".into(); }
                                    2 => { #[cfg(feature = "http")] { match net::explain_approval(&id).await { Ok(s)=> app.ep_explain=Some(s), Err(e)=> app.ep_explain=Some(format!("explain error: {}", e)), } } }
                                    _ => {}
                                }
                            }
                            _ => {}
                        }
                        continue;
                    }
                    match crate::keymap::resolve(k) {
                        crate::keymap::Hotkey::Quit => break Ok(()),
                        crate::keymap::Hotkey::SwitchTab(i) => { app.active = match i { 0=>Screen::Chat,1=>Screen::Dashboard,2=>Screen::Tasks,3=>Screen::Memory,4=>Screen::Tools,5=>Screen::Codex,6=>Screen::Reports,7=>Screen::Research,_=>Screen::Settings }; },
                        crate::keymap::Hotkey::ToggleHelp => { app.show_help = !app.show_help; },
                        crate::keymap::Hotkey::Refresh => { refresh_current(&mut app).await; },
                        crate::keymap::Hotkey::VoicePTT => {
                            if !app.voice_ptt {
                                #[cfg(feature = "voice")]
                                {
                                    match crate::audio::VoiceRecorder::start() {
                                        Ok(r) => { app.voice_rec = Some(r); app.voice_ptt = true; app.voice_last_seen = Some(Instant::now()); app.status = "Listening… (hold Ctrl-Space)".into(); push_toast(&mut app, "Listening…", ToastKind::Info); }
                                        Err(e) => { push_toast(&mut app, format!("Mic error: {}", e), ToastKind::Error); }
                                    }
                                }
                                #[cfg(not(feature = "voice"))]
                                { push_toast(&mut app, "Voice feature not enabled (build with --features voice)", ToastKind::Warn); }
                            } else {
                                // Treat repeated Presses as keepalive when already listening
                                app.voice_last_seen = Some(Instant::now());
                            }
                        }
                        crate::keymap::Hotkey::VoiceHangup => {
                            #[cfg(feature = "http")]
                            {
                                match net::realtime_stop().await { Ok(()) => { app.status = "Voice: call ended".into(); app.rt_active = false; push_toast(&mut app, "Realtime: Ended", ToastKind::Info); }, Err(e) => { push_toast(&mut app, format!("Hangup error: {}", e), ToastKind::Error); } }
                            }
                        }
                        crate::keymap::Hotkey::OpenProjectPicker => {
                            // Populate project list and open modal
                            let home = std::env::var("HOME").unwrap_or_else(|_| "".into());
                            let dev = std::path::Path::new(&home).join("dev");
                            let games = std::path::Path::new(&home).join("games");
                            let mut entries: Vec<String> = vec![];
                            for root in [dev, games] {
                                if let Ok(rd) = std::fs::read_dir(&root) {
                                    for e in rd.flatten() {
                                        if let Ok(md) = e.metadata() { if md.is_dir() {
                                            if let Ok(p) = e.path().into_os_string().into_string() { entries.push(p); }
                                        } }
                                    }
                                }
                            }
                            entries.sort();
                            app.proj_list = entries;
                            app.proj_sel = 0;
                            app.proj_modal_open = true;
                            app.status = "Select a project (↑/↓, Enter, Esc)".into();
                        }
                        crate::keymap::Hotkey::PickSteamGame => {
                            if let Screen::Tools = app.active {
                                let server = app.tools_list.get(app.tool_server_sel).map(|(s, _)| s.clone()).unwrap_or_default();
                                let tools = app.tools_list.get(app.tool_server_sel).map(|(_, t)| t.clone()).unwrap_or_default();
                                let tool = tools.get(app.tool_tool_sel).cloned().unwrap_or_default();
                                if server == "steam" && tool == "launch" {
                                    let list = load_steamgames_list();
                                    if list.is_empty() {
                                        push_toast(&mut app, "No Steam games in config/steamgames.toml", ToastKind::Warn);
                                    } else {
                                        app.steam_list = list;
                                        app.steam_sel = 0;
                                        app.steam_modal_open = true;
                                        app.status = "Pick a Steam game (↑/↓, Enter, Esc)".into();
                                    }
                                } else {
                                    push_toast(&mut app, "PickSteam only for steam.launch tool", ToastKind::Warn);
                                }
                            }
                        }
                        crate::keymap::Hotkey::CodexNew => {
                            if let Screen::Codex = app.active {
                                #[cfg(feature = "http")]
                                {
                                    let prompt = app.input.trim().to_string();
                                    if prompt.is_empty() { push_toast(&mut app, "Type a prompt in Input", ToastKind::Warn); }
                                    else {
                                        let repo = app.proj_current.as_deref();
                                        match net::codex_new(&prompt, repo).await {
                                            Ok(sid) => { app.status = format!("Codex new: {}", sid); push_toast(&mut app, "Codex session started", ToastKind::Success); let _ = reload_codex(&mut app).await; }
                                            Err(e) => { push_toast(&mut app, format!("Codex new error: {}", e), ToastKind::Error); }
                                        }
                                    }
                                }
                            }
                        }
                        crate::keymap::Hotkey::CodexContinue => {
                            if let Screen::Codex = app.active {
                                #[cfg(feature = "http")]
                                {
                                    if let Some(s) = app.codex_sessions.get(app.codex_sel).cloned() {
                                        let prompt = app.input.trim().to_string();
                                        if prompt.is_empty() { push_toast(&mut app, "Type a prompt in Input", ToastKind::Warn); }
                                        else {
                                            match net::codex_continue(&s.session_id, &prompt).await {
                                                Ok(_) => { app.status = "Codex continued".into(); push_toast(&mut app, "Codex continued", ToastKind::Success); let _ = reload_codex_detail(&mut app).await; }
                                                Err(e) => { push_toast(&mut app, format!("Codex continue error: {}", e), ToastKind::Error); }
                                            }
                                        }
                                    } else { push_toast(&mut app, "No session selected", ToastKind::Warn); }
                                }
                            }
                        }
                        crate::keymap::Hotkey::FocusSearch => {
                            if let Screen::Memory = app.active {
                                app.focus_ix = 0; // search
                                app.input = app.mem_query.clone();
                                app.status = "Search focus".into();
                            }
                        }
                        crate::keymap::Hotkey::TogglePin => {
                            #[cfg(feature = "http")]
                            {
                                if let Screen::Memory = app.active {
                                    if let Some(atom_id) = current_memory_atom_id(&app) {
                                        let want_pin = !app.mem_atom.as_ref().map(|a| a.pinned).unwrap_or(false);
                                        let res = if want_pin { net::pin_atom(atom_id).await } else { net::unpin_atom(atom_id).await };
                                        match res {
                                            Ok(_) => { app.status = if want_pin { "Pinned".into() } else { "Unpinned".into() }; push_toast(&mut app, if want_pin { "Pinned" } else { "Unpinned" }, ToastKind::Success); reload_memory_detail(&mut app, atom_id).await; }
                                            Err(e) => { app.status = format!("pin err: {}", e); push_toast(&mut app, format!("Pin error: {}", e), ToastKind::Error); }
                                        }
                                    }
                                }
                            }
                        }
                        crate::keymap::Hotkey::OpenReport => {
                            #[cfg(feature = "http")]
                            {
                                if let Screen::Reports = app.active {
                                    if let Some(name) = app.reports.get(app.report_sel).cloned() {
                                        match net::read_report(&name).await {
                                            Ok(c) => { app.report_content = c; app.status = format!("opened {}", name); push_toast(&mut app, format!("Opened {}", name), ToastKind::Info); },
                                            Err(e) => { app.status = format!("open err: {}", e); push_toast(&mut app, format!("Open error: {}", e), ToastKind::Error); }
                                        }
                                    }
                                } else if let Screen::Memory = app.active {
                                    // Open details for selected hit
                                    open_memory_detail(&mut app).await;
                                }
                            }
                        }
                        crate::keymap::Hotkey::EditToolParams => {
                            if let Screen::Tools = app.active {
                                app.editing_params = true;
                                app.input = app.tool_params.clone();
                                push_toast(&mut app, "Editing params (Enter to save)", ToastKind::Info);
                            }
                        }
                        crate::keymap::Hotkey::None => {}
                    }
                    match (k.modifiers, k.code) {
                        (KeyModifiers::NONE, KeyCode::Esc) => { app.show_help = false; },
                        (KeyModifiers::NONE, KeyCode::Up) => {
                            match app.active {
                                Screen::Reports => { if app.report_sel > 0 { app.report_sel -= 1; } }
                                Screen::Tools => { if app.tool_tool_sel > 0 { app.tool_tool_sel -= 1; } }
                                Screen::Memory => { if app.mem_sel > 0 { app.mem_sel -= 1; } }
                                Screen::Codex => { if app.codex_sel > 0 { app.codex_sel -= 1; } }
                                Screen::Research => { if app.research_sel > 0 { app.research_sel -= 1; } }
                                _ => {}
                            }
                        },
                        (KeyModifiers::NONE, KeyCode::Down) => {
                            match app.active {
                                Screen::Reports => { if !app.reports.is_empty() { app.report_sel = (app.report_sel + 1).min(app.reports.len()-1); } }
                                Screen::Tools => {
                                    let tools = app.tools_list.get(app.tool_server_sel).map(|(_, t)| t).cloned().unwrap_or_default();
                                    if !tools.is_empty() { app.tool_tool_sel = (app.tool_tool_sel + 1).min(tools.len()-1); }
                                }
                                Screen::Memory => { if !app.mem_results.is_empty() { app.mem_sel = (app.mem_sel + 1).min(app.mem_results.len()-1); } }
                                Screen::Codex => { if !app.codex_sessions.is_empty() { app.codex_sel = (app.codex_sel + 1).min(app.codex_sessions.len()-1); } }
                                Screen::Research => { if !app.research_results.is_empty() { app.research_sel = (app.research_sel + 1).min(app.research_results.len()-1); } }
                                _ => {}
                            }
                        },
                        // Plain-letter handlers removed to avoid stealing typing
                        (KeyModifiers::NONE, KeyCode::Left) => {
                            match app.active {
                                Screen::Tools => { if app.tool_server_sel > 0 { app.tool_server_sel -= 1; app.tool_tool_sel = 0; } }
                                Screen::Research => { if app.focus_ix > 0 { app.focus_ix -= 1; } }
                                _ => {}
                            }
                        },
                        (KeyModifiers::NONE, KeyCode::Right) => {
                            match app.active {
                                Screen::Tools => { if !app.tools_list.is_empty() { app.tool_server_sel = (app.tool_server_sel + 1).min(app.tools_list.len()-1); app.tool_tool_sel = 0; } }
                                Screen::Research => { let max = 2; if (app.focus_ix as usize) < max { app.focus_ix += 1; } }
                                _ => {}
                            }
                        },
                        (KeyModifiers::CONTROL, KeyCode::Char('k')) => { app.input.clear(); },
                        (KeyModifiers::NONE, KeyCode::Tab) => {
                            let max = match app.active { Screen::Chat => 1, Screen::Dashboard => 3, Screen::Reports => 2, Screen::Tools => 4, Screen::Memory => 4, Screen::Codex => 2, Screen::Research => 3, _ => 1 };
                            app.focus_ix = (app.focus_ix + 1) % max;
                        }
                        (KeyModifiers::SHIFT, KeyCode::BackTab) | (KeyModifiers::SHIFT, KeyCode::Tab) => {
                            let max = match app.active { Screen::Chat => 1, Screen::Dashboard => 3, Screen::Reports => 2, Screen::Tools => 4, Screen::Memory => 4, Screen::Codex => 2, Screen::Research => 3, _ => 1 };
                            app.focus_ix = (app.focus_ix + max - 1) % max;
                        }
                        // Research screen quick actions (guarded to not steal typing elsewhere)
                        (KeyModifiers::NONE, KeyCode::Char('d')) if app.active == Screen::Research => {
                            #[cfg(feature = "http")]
                            {
                                if let Some(id) = app.research_results.get(app.research_sel).and_then(|row| row.split('|').next()).map(|s| s.trim().to_string()) {
                                    let params = serde_json::json!({"id": id});
                                    match net::call_tool("arxiv", "fetch_pdf", &params.to_string()).await {
                                        Ok(_) => push_toast(&mut app, "Downloaded PDF", ToastKind::Success),
                                        Err(e) => push_toast(&mut app, format!("download error: {}", e), ToastKind::Error),
                                    }
                                }
                            }
                        }
                        (KeyModifiers::NONE, KeyCode::Char('b')) if app.active == Screen::Research => {
                            #[cfg(feature = "http")]
                            {
                                match net::run_job("arxiv").await { Ok(()) => push_toast(&mut app, "arXiv brief started", ToastKind::Success), Err(e) => push_toast(&mut app, format!("brief error: {}", e), ToastKind::Error) }
                            }
                        }
                        (KeyModifiers::NONE, KeyCode::Enter) => {
                            if let Screen::Research = app.active {
                                #[cfg(feature = "http")]
                                {
                                    if app.input.trim().is_empty() {
                                        if let Some(id) = app.research_results.get(app.research_sel).and_then(|row| row.split('|').next()).map(|s| s.trim().to_string()) {
                                            let params = serde_json::json!({"id": id});
                                            match net::call_tool("arxiv", "summarize", &params.to_string()).await {
                                                Ok(s) => { app.research_details = s; }
                                                Err(e) => push_toast(&mut app, format!("summarize error: {}", e), ToastKind::Error),
                                            }
                                        }
                                    } else {
                                        let q = app.input.trim().to_string();
                                        let params = serde_json::json!({"query": q, "max_results": 25});
                                        match net::call_tool("arxiv", "search", &params.to_string()).await {
                                            Ok(s) => {
                                                if let Ok(v) = serde_json::from_str::<serde_json::Value>(&s) {
                                                    let rows = v.get("results").and_then(|x| x.as_array()).cloned().unwrap_or_default();
                                                    let list: Vec<String> = rows.into_iter().map(|r| {
                                                        let id = r.get("id").and_then(|x| x.as_str()).unwrap_or("");
                                                        let title = r.get("title").and_then(|x| x.as_str()).unwrap_or("");
                                                        format!("{} | {}", id, title)
                                                    }).collect();
                                                    app.research_results = list;
                                                    app.research_sel = 0;
                                                    app.research_details.clear();
                                                    app.status = "arXiv: search complete".into();
                                                }
                                            }
                                            Err(e) => push_toast(&mut app, format!("search error: {}", e), ToastKind::Error),
                                        }
                                    }
                                }
                                continue;
                            }
                            if app.editing_params {
                                app.tool_params = app.input.clone();
                                app.editing_params = false;
                                app.status = "params updated".into();
                                push_toast(&mut app, "Params updated", ToastKind::Success);
                                app.input.clear();
                            } else if let Screen::Reports = app.active {
                                #[cfg(feature = "http")]
                                {
                                    if let Some(name) = app.reports.get(app.report_sel).cloned() {
                                        match net::read_report(&name).await {
                                            Ok(c) => { app.report_content = c; app.status = format!("opened {}", name); push_toast(&mut app, format!("Opened {}", name), ToastKind::Info); },
                                            Err(e) => { app.status = format!("open err: {}", e); push_toast(&mut app, format!("Open error: {}", e), ToastKind::Error); }
                                        }
                                    }
                                }
                            } else if let Screen::Tools = app.active {
                                #[cfg(feature = "http")]
                                {
                                    if let Some((server, tools)) = app.tools_list.get(app.tool_server_sel).cloned() {
                                        if let Some(tool) = tools.get(app.tool_tool_sel).cloned() {
                                            match net::call_tool(&server, &tool, &app.tool_params).await {
                                                Ok(out) => { app.tool_output_text = out; app.status = format!("ran {}.{}", server, tool); push_toast(&mut app, format!("Ran {}.{}", server, tool), ToastKind::Success); },
                                                Err(e) => { app.tool_output_text.clear(); app.status = format!("tool err: {}", e); push_toast(&mut app, format!("Tool error: {}", e), ToastKind::Error); }
                                            }
                                        }
                                    }
                                }
                            } else if app.active == Screen::Chat && !app.input.trim().is_empty() && !app.input.trim().starts_with('/') {
                                // Send chat message to OpenAI
                                let user = app.input.trim().to_string();
                                app.chat_messages.push(ChatMsg { role: "user".into(), content: user.clone() });
                                app.chat_scroll = 0;
                                app.input.clear();
                                #[cfg(feature = "http")]
                                {
                                    if app.chat_session_id.is_none() {
                                        match net::chat_new().await { Ok(id) => app.chat_session_id = Some(id), Err(e) => push_toast(&mut app, format!("chat new err: {}", e), ToastKind::Error) }
                                    }
                                    if let Some(id) = app.chat_session_id.clone() {
                                        let _ = net::chat_append(&id, "user", &user).await;
                                    }
                                    let history = app.chat_messages.clone();
                                    let tx = evt_tx.clone();
                                    if app.chat_stream {
                                        // Insert placeholder assistant message for streaming
                                        app.chat_messages.push(ChatMsg { role: "assistant".into(), content: String::new() });
                                        let sid = app.chat_session_id.clone();
                                        let model = app.chat_model.clone();
                                        tokio::spawn(async move {
                                            let tx2 = tx.clone();
                                            let r = net::core_chat_stream(&history, &model, move |event, data| {
                                                match event.as_str() {
                                                    "token" => { let _ = tx.send(ChatEvent::StreamDelta(data.clone())); }
                                                    "tool_call" => { let _ = tx.send(ChatEvent::ToolNote(format!("Tool call: {}", data))); }
                                                    "tool_result" => { let _ = tx.send(ChatEvent::ToolNote(format!("Tool result: {}", data))); }
                                                    "error" => { let _ = tx.send(ChatEvent::Error(format!("{}", data))); }
                                                    "done" => { let _ = tx.send(ChatEvent::StreamDone); }
                                                    _ => {}
                                                }
                                            }).await;
                                            match r {
                                                Ok(final_text) => {
                                                    if let Some(id) = sid { let _ = net::chat_append(&id, "assistant", &final_text).await; }
                                                }
                                                Err(e) => { let _ = tx2.send(ChatEvent::Error(format!("Chat error: {}", e))); }
                                            }
                                        });
                                    } else {
                                        let sid = app.chat_session_id.clone();
                                        let model = app.chat_model.clone();
                                        tokio::spawn(async move {
                                            match net::core_chat_complete(&history, &model).await {
                                                Ok(reply) => {
                                                    let _ = tx.send(ChatEvent::AssistantReply(reply.clone()));
                                                    if let Some(id) = sid { let _ = net::chat_append(&id, "assistant", &reply).await; }
                                                }
                                                Err(e) => { let _ = tx.send(ChatEvent::Error(format!("Chat error: {}", e))); }
                                            }
                                        });
                                    }
                                }
                            } else if let Screen::Codex = app.active {
                                #[cfg(feature = "http")]
                                {
                                    let prompt = app.input.trim().to_string();
                                    if prompt.is_empty() {
                                        push_toast(&mut app, "Type a prompt in Input", ToastKind::Warn);
                                    } else {
                                        if let Some(s) = app.codex_sessions.get(app.codex_sel).cloned() {
                                            // Local echo for responsiveness; server will append actual events
                                            if app.codex_detail.is_empty() { app.codex_detail = format!("you: {}", prompt); }
                                            else { app.codex_detail = format!("{}\n\nyou: {}", app.codex_detail, prompt); }
                                            match net::codex_continue(&s.session_id, &prompt).await {
                                                Ok(_) => { let _ = reload_codex_detail(&mut app).await; push_toast(&mut app, "Sent to Codex", ToastKind::Info); }
                                                Err(e) => { push_toast(&mut app, format!("Codex continue error: {}", e), ToastKind::Error); }
                                            }
                                        } else {
                                            let repo = app.proj_current.as_deref();
                                            match net::codex_new(&prompt, repo).await {
                                                Ok(sid) => { push_toast(&mut app, "Codex session started", ToastKind::Success); let _ = reload_codex(&mut app).await; if !sid.is_empty() { app.status = format!("Codex new: {}", sid); } }
                                                Err(e) => { push_toast(&mut app, format!("Codex new error: {}", e), ToastKind::Error); }
                                            }
                                        }
                                        app.input.clear();
                                    }
                                }
                            } else if let Screen::Memory = app.active {
                                #[cfg(feature = "http")]
                                {
                                    if app.focus_ix == 0 {
                                        // run search
                                        app.mem_query = app.input.trim().to_string();
                                        match net::memory_search(&app.mem_query, None, 25).await {
                                            Ok(hits) => { app.mem_results = hits; app.mem_sel = 0; app.status = "search ok".into(); push_toast(&mut app, "Search OK", ToastKind::Success); }
                                            Err(e) => { app.status = format!("search err: {}", e); push_toast(&mut app, format!("Search error: {}", e), ToastKind::Error); }
                                        }
                                        // Pack preview
                                        match net::context_pack(None, 2048, 12, vec![]).await {
                                            Ok(sum) => { app.mem_pack_summary = sum; }
                                            Err(e) => { app.mem_pack_summary = format!("pack err: {}", e); }
                                        }
                                    } else if app.focus_ix == 1 {
                                        open_memory_detail(&mut app).await;
                                    }
                                }
                            } else {
                                let cmd = app.input.trim().to_string();
                                handle_command(&mut app, cmd).await;
                                app.input.clear();
                            }
                        }
                        (KeyModifiers::NONE, KeyCode::Char(ch)) => {
                            // Always treat unmodified characters as input typing
                            // (tab switching uses F-keys or Ctrl/Alt digits via keymap.rs)
                            app.input.push(ch)
                        },
                        (KeyModifiers::NONE, KeyCode::Backspace) => { app.input.pop(); },
                        _ => {}
                    }
                }
                Event::Key(k) if k.kind == KeyEventKind::Release => {
                    // Stop recording as soon as Space is released (regardless of Ctrl state)
                    if matches!(k.code, KeyCode::Char(' ')) {
                        if app.voice_ptt {
                            app.voice_ptt = false;
                            app.voice_last_seen = None;
                            #[cfg(feature = "voice")]
                            {
                                if let Some(rec) = app.voice_rec.take() {
                                    let tx = evt_tx.clone();
                                    tokio::spawn(async move {
                                        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
                                        match rec.stop_and_into_wav() {
                                            Ok(wav) => {
                                                match net::stt_transcribe_whisper(wav).await {
                                                    Ok(text) => { let _ = tx.send(ChatEvent::VoiceTranscript(text)); },
                                                    Err(e) => { let _ = tx.send(ChatEvent::Error(format!("stt error: {}", e))); }
                                                }
                                            }
                                            Err(e) => { let _ = tx.send(ChatEvent::Error(format!("mic stop error: {}", e))); }
                                        }
                                    });
                                    push_toast(&mut app, "Processing…", ToastKind::Info);
                                    app.status = "Processing voice…".into();
                                }
                            }
                        }
                    }
                }
                Event::Key(k) if k.kind == KeyEventKind::Repeat => {
                    // Refresh last-seen while Ctrl+Space is held (for terminals that repeat keys)
                    if k.modifiers.contains(KeyModifiers::CONTROL) && matches!(k.code, KeyCode::Char(' ')) {
                        if app.voice_ptt { app.voice_last_seen = Some(Instant::now()); }
                    }
                }
                Event::Mouse(m) => {
                    use crossterm::event::MouseEventKind;
                    match m.kind {
                        MouseEventKind::ScrollUp => { if app.active == Screen::Chat { app.chat_scroll = app.chat_scroll.saturating_add(3); } }
                        MouseEventKind::ScrollDown => { if app.active == Screen::Chat { app.chat_scroll = app.chat_scroll.saturating_sub(3); } }
                        _ => {}
                    }
                }
                _ => {}
            }
        }
        #[cfg(feature = "http")]
        {
            if app.ep_prompt.is_none() {
                if let Ok(Some(p)) = net::get_approval_prompt().await { app.ep_prompt = Some(p); app.ep_sel=0; app.ep_explain=None; }
            }
        }

        // Voice PTT idle detection: if no repeats/presses for ~700ms, stop
        if app.voice_ptt {
            if let Some(last) = app.voice_last_seen {
                if Instant::now().duration_since(last) > Duration::from_millis(700) {
                    app.voice_ptt = false;
                    app.voice_last_seen = None;
                    #[cfg(feature = "voice")]
                    {
                        if let Some(rec) = app.voice_rec.take() {
                            let tx = evt_tx.clone();
                            tokio::spawn(async move {
                                tokio::time::sleep(std::time::Duration::from_millis(300)).await;
                                match rec.stop_and_into_wav() {
                                    Ok(wav) => {
                                        match net::stt_transcribe_whisper(wav).await {
                                            Ok(text) => { let _ = tx.send(ChatEvent::VoiceTranscript(text)); },
                                            Err(e) => { let _ = tx.send(ChatEvent::Error(format!("stt error: {}", e))); }
                                        }
                                    }
                                    Err(e) => { let _ = tx.send(ChatEvent::Error(format!("mic stop error: {}", e))); }
                                }
                            });
                            push_toast(&mut app, "Processing…", ToastKind::Info);
                            app.status = "Processing voice…".into();
                        }
                    }
                }
            }
        }

        // Apply pending mouse capture toggle so user can select/copy
        if app.want_mouse_capture != app.mouse_capture {
            if app.want_mouse_capture {
                crossterm::execute!(terminal.backend_mut(), crossterm::event::EnableMouseCapture)?;
            } else {
                crossterm::execute!(terminal.backend_mut(), crossterm::event::DisableMouseCapture)?;
            }
            app.mouse_capture = app.want_mouse_capture;
        }
    };

    disable_raw_mode()?;
    crossterm::execute!(terminal.backend_mut(), crossterm::event::DisableMouseCapture, crossterm::terminal::LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    res
}

async fn handle_command(mut app: &mut App, cmd: String) {
    if cmd == "/help" {
        app.status = "Commands: /help, /new, /load, /delete, /stream on|off, /models [gpt-5|gpt-5-mini], /select on|off, /voice start [ws://…]|status|end|enable_wake|disable_wake, /audio test|devices|beep".into();
        return;
    }
    if let Some(rest) = cmd.strip_prefix("/select") {
        let arg = rest.trim();
        if arg.is_empty() {
            app.status = format!("Selection: {} (use /select on|off)", if app.want_mouse_capture { "off" } else { "on" });
            return;
        }
        match arg {
            "on" => { app.want_mouse_capture = false; app.status = "Selection: on (mouse capture disabled)".into(); push_toast(&mut app, "Selection mode ON", ToastKind::Info); }
            "off" => { app.want_mouse_capture = true; app.status = "Selection: off (mouse capture enabled)".into(); push_toast(&mut app, "Selection mode OFF", ToastKind::Info); }
            _ => { app.status = "usage: /select on|off".into(); }
        }
        return;
    }
    if let Some(rest) = cmd.strip_prefix("/models") {
        let arg = rest.trim();
        if arg.is_empty() {
            app.status = format!("model: {} (available: gpt-5, gpt-5-mini)", app.chat_model);
            return;
        }
        let choice = arg.to_ascii_lowercase();
        match choice.as_str() {
            "gpt-5" | "gpt-5-mini" => {
                app.chat_model = choice;
                let model_set = app.chat_model.clone();
                push_toast(&mut app, format!("Model set: {}", model_set), ToastKind::Success);
                app.status = format!("model: {}", app.chat_model);
            }
            _ => {
                app.status = "usage: /models [gpt-5|gpt-5-mini]".into();
            }
        }
        return;
    }
    if let Some(rest) = cmd.strip_prefix("/theme ") {
        match rest.trim() {
            "toggle" => { theme::toggle_mode(); push_toast(&mut app, format!("Theme: {}", theme::mode_name()), ToastKind::Info); }
            "amber" => { theme::set_mode(theme::Mode::AmberDark); push_toast(&mut app, "Theme: AmberDark", ToastKind::Info); }
            "default" => { theme::set_mode(theme::Mode::Default); push_toast(&mut app, "Theme: Default", ToastKind::Info); }
            _ => { app.status = "usage: /theme [toggle|amber|default]".into(); }
        }
        return;
    }
    if cmd == "/voice test" {
        #[cfg(feature = "http")]
        {
            match net::audio_diagnose(None).await {
                Ok(s) => { app.status = s.clone(); app.tool_output_text = s; push_toast(&mut app, "Audio diagnose complete", ToastKind::Success); },
                Err(e) => app.status = format!("audio diag err: {}", e),
            }
            return;
        }
        #[cfg(not(feature = "http"))]
        { app.status = "voice test (http feature disabled)".into(); return; }
    }
    if cmd == "/audio test" {
        #[cfg(feature = "http")]
        {
            match net::audio_diagnose(Some(true)).await {
                Ok(s) => { app.status = s.clone(); app.tool_output_text = s; push_toast(&mut app, "Audio test complete", ToastKind::Success); },
                Err(e) => { app.status = format!("audio test err: {}", e); push_toast(&mut app, format!("Audio test error: {}", e), ToastKind::Error); },
            }
            return;
        }
    }
    if cmd == "/audio devices" {
        #[cfg(feature = "http")]
        {
            match net::audio_devices().await {
                Ok(s) => { app.status = s.clone(); app.tool_output_text = s; push_toast(&mut app, "Audio devices fetched", ToastKind::Info); },
                Err(e) => { app.status = format!("audio devices err: {}", e); push_toast(&mut app, format!("Audio devices error: {}", e), ToastKind::Error); },
            }
            return;
        }
    }
    if cmd == "/audio beep" {
        #[cfg(feature = "http")]
        {
            match net::audio_beep(None).await { Ok(()) => { app.status = "beep".into(); }, Err(e) => { app.status = format!("beep err: {}", e); push_toast(&mut app, format!("Beep error: {}", e), ToastKind::Error); } }
            return;
        }
    }
    if let Some(rest) = cmd.strip_prefix("/voice start") {
        #[cfg(feature = "http")]
        {
            let endpoint = rest.trim();
            let endpoint_opt = if endpoint.is_empty() { None } else { Some(endpoint.to_string()) };
            match net::realtime_start(endpoint_opt).await {
                Ok(()) => { app.rt_active = true; app.status = "realtime: starting".into(); push_toast(&mut app, "Realtime: Connecting…", ToastKind::Info); },
                Err(e) => { app.status = format!("realtime start err: {}", e); push_toast(&mut app, format!("Realtime start error: {}", e), ToastKind::Error); }
            }
            return;
        }
    }
    if cmd == "/voice status" {
        #[cfg(feature = "http")]
        {
            let (rts, err) = net::realtime_status().await.unwrap_or((false, None));
            let ws = net::wake_status().await.unwrap_or(serde_json::json!({}));
            let wd = net::wake_daemon_status().await.unwrap_or(serde_json::json!({}));
            app.rt_active = rts;
            let rt = if rts {"active"} else {"idle"};
            let err_txt = err.map(|e| format!(" • err: {}", e)).unwrap_or_default();
            app.status = format!("realtime: {}{} • wake(core): {} • wake(daemon): {}", rt, err_txt, ws, wd);
            push_toast(&mut app, "Updated voice status", ToastKind::Info);
            return;
        }
    }
    if cmd == "/voice end" {
        #[cfg(feature = "http")]
        {
            match net::realtime_stop().await { Ok(()) => { app.rt_active = false; app.status = "realtime: ended".into(); push_toast(&mut app, "Realtime: Ended", ToastKind::Info); }, Err(e) => { app.status = format!("realtime stop err: {}", e); push_toast(&mut app, format!("Realtime stop error: {}", e), ToastKind::Error); } }
            return;
        }
    }
    if cmd == "/voice enable_wake" {
        #[cfg(feature = "http")]
        {
            match net::wake_enable().await { Ok(()) => { app.status = "wake: enabled".into(); push_toast(&mut app, "Wake enabled", ToastKind::Success); }, Err(e) => { app.status = format!("wake enable err: {}", e); push_toast(&mut app, format!("Wake enable error: {}", e), ToastKind::Error); } }
            return;
        }
    }
    if cmd == "/voice disable_wake" {
        #[cfg(feature = "http")]
        {
            match net::wake_disable().await { Ok(()) => { app.status = "wake: disabled".into(); push_toast(&mut app, "Wake disabled", ToastKind::Info); }, Err(e) => { app.status = format!("wake disable err: {}", e); push_toast(&mut app, format!("Wake disable error: {}", e), ToastKind::Error); } }
            return;
        }
    }
    if let Some(rest) = cmd.strip_prefix("/logs") {
        let n: usize = rest.trim().split_whitespace().next().and_then(|s| if s.is_empty() { None } else { s.parse().ok() }).unwrap_or(200);
        let path = std::path::Path::new("storage/logs/ui-tui.log");
        match std::fs::read_to_string(path) {
            Ok(content) => {
                let mut lines: Vec<&str> = content.lines().collect();
                if lines.len() > n { lines = lines[lines.len()-n..].to_vec(); }
                let out = lines.join("\n");
                app.tool_output_text = out;
                app.active = Screen::Tools;
                app.status = format!("logs: last {} lines", n);
                push_toast(&mut app, "Opened logs in Output", ToastKind::Info);
            }
            Err(e) => { app.status = format!("logs err: {}", e); push_toast(&mut app, format!("Logs error: {}", e), ToastKind::Error); }
        }
        return;
    }
    if let Some(_) = cmd.strip_prefix("/proj scan") {
        let home = std::env::var("HOME").unwrap_or_else(|_| "".into());
        let dev = std::path::Path::new(&home).join("dev");
        let games = std::path::Path::new(&home).join("games");
        let mut entries: Vec<String> = vec![];
        for root in [dev, games] {
            if let Ok(rd) = std::fs::read_dir(&root) {
                for e in rd.flatten() {
                    if let Ok(md) = e.metadata() { if md.is_dir() {
                        if let Ok(p) = e.path().into_os_string().into_string() { entries.push(p); }
                    } }
                }
            }
        }
        entries.sort();
        app.proj_list = entries;
        app.proj_sel = 0;
        app.proj_modal_open = true;
        app.status = "Select a project (↑/↓, Enter, Esc)".into();
        return;
    }
    if let Some(rest) = cmd.strip_prefix("/proj pick ") {
        let path = rest.trim();
        if path.is_empty() { app.status = "usage: /proj pick <path>".into(); return; }
        let p = std::path::Path::new(path);
        if p.is_dir() {
            app.proj_current = Some(path.to_string());
            push_toast(&mut app, format!("Project set: {}", path), ToastKind::Success);
            app.status = format!("Project: {}", path);
        } else {
            app.status = format!("not a dir: {}", path);
            push_toast(&mut app, format!("Not a directory: {}", path), ToastKind::Warn);
        }
        return;
    }
    if let Some(rest) = cmd.strip_prefix("/proj new ") {
        // usage: /proj new <name> [dev|games]
        let mut parts = rest.split_whitespace();
        let name = parts.next().unwrap_or("");
        let kind = parts.next().unwrap_or("dev");
        if name.is_empty() { app.status = "usage: /proj new <name> [dev|games]".into(); return; }
        #[cfg(feature = "http")]
        {
            let params = serde_json::json!({"name": name, "kind": kind, "git": true});
            match net::call_tool("project", "init", &params.to_string()).await {
                Ok(out) => {
                    // Parse path from result JSON
                    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&out) {
                        if let Some(p) = v.get("path").and_then(|x| x.as_str()) {
                            app.proj_current = Some(p.to_string());
                            app.status = format!("Project created: {}", p);
                            push_toast(&mut app, format!("Project created: {}", p), ToastKind::Success);
                        } else {
                            app.status = "project created (path unknown)".into();
                        }
                    } else {
                        app.status = "project created".into();
                    }
                }
                Err(e) => { app.status = format!("proj err: {}", e); push_toast(&mut app, format!("Project error: {}", e), ToastKind::Error); }
            }
        }
        return;
    }
    if cmd == "/proj clear" {
        app.proj_current = None;
        app.status = "Project cleared".into();
        push_toast(&mut app, "Project cleared", ToastKind::Info);
        return;
    }
    if let Some(rest) = cmd.strip_prefix("/ls ") {
        #[cfg(feature = "http")]
        {
            match net::tool_shell_list_dir(rest).await {
                Ok(list) => { app.tool_output = list; app.active = Screen::Tools; app.status = "ok".into(); }
                Err(e) => app.status = format!("ls err: {}", e),
            }
            return;
        }
        #[cfg(not(feature = "http"))]
        { app.status = "ls unsupported".into(); return; }
    }
    if let Some(rest) = cmd.strip_prefix("/tool ") {
        #[cfg(feature = "http")]
        {
            // Accept formats: "/tool server tool {json}" or "/tool server.tool {json}"
            let mut server = String::new();
            let mut tool = String::new();
            let mut params = "{}".to_string();
            let trimmed = rest.trim();
            if let Some((st, p)) = trimmed.split_once(' ') {
                // st may be "server tool" or "server.tool"
                if let Some((s, t)) = st.split_once('.') {
                    server = s.to_string();
                    tool = t.to_string();
                    params = p.trim().to_string();
                } else {
                    let mut parts = st.split_whitespace();
                    server = parts.next().unwrap_or("").to_string();
                    tool = parts.next().unwrap_or("").to_string();
                    params = p.trim().to_string();
                }
            } else {
                // Only st provided; no params
                if let Some((s, t)) = trimmed.split_once('.') {
                    server = s.to_string();
                    tool = t.to_string();
                } else {
                    let mut parts = trimmed.split_whitespace();
                    server = parts.next().unwrap_or("").to_string();
                    tool = parts.next().unwrap_or("").to_string();
                }
            }
            if server.is_empty() || tool.is_empty() { app.status = "usage: /tool <server>.<tool> {json}".into(); return; }
            match net::call_tool(&server, &tool, &params).await {
                Ok(out) => { app.status = format!("ok: {}.{}", server, tool); push_toast(&mut app, format!("Ran {}.{}", server, tool), ToastKind::Success); app.active = Screen::Tools; app.tool_output_text = out; }
                Err(e) => { app.status = format!("tool err: {}", e); push_toast(&mut app, format!("Tool error: {}", e), ToastKind::Error); }
            }
            return;
        }
        #[cfg(not(feature = "http"))]
        { app.status = "tool unsupported".into(); return; }
    }
    if cmd == "/approvals" {
        #[cfg(feature = "http")]
        {
            match net::list_approvals().await {
                Ok(aps) => { app.approvals = aps; app.active = Screen::Tools; app.status = "approvals ok".into(); }
                Err(e) => app.status = format!("approvals err: {}", e),
            }
            return;
        }
        #[cfg(not(feature = "http"))]
        { app.status = "approvals unsupported".into(); return; }
    }
    if cmd == "/new" {
        #[cfg(feature = "http")]
        {
            match net::chat_new().await {
                Ok(id) => { app.chat_session_id = Some(id.clone()); app.chat_messages.clear(); app.chat_scroll = 0; app.status = format!("New chat {}", &id[..8]); push_toast(&mut app, "New chat", ToastKind::Success); }
                Err(e) => { app.status = format!("new err: {}", e); push_toast(&mut app, format!("New chat error: {}", e), ToastKind::Error); }
            }
            return;
        }
    }
    if cmd == "/load" {
        #[cfg(feature = "http")]
        {
            match net::chat_latest().await {
                Ok(Some(s)) => { app.chat_session_id = Some(s.id.clone()); app.chat_messages = s.messages; app.chat_scroll = 0; app.status = format!("Loaded {}", &s.id[..8]); push_toast(&mut app, "Loaded latest chat", ToastKind::Info); }
                Ok(None) => { app.status = "no sessions".into(); push_toast(&mut app, "No sessions to load", ToastKind::Warn); }
                Err(e) => { app.status = format!("load err: {}", e); push_toast(&mut app, format!("Load error: {}", e), ToastKind::Error); }
            }
            return;
        }
    }
    if cmd == "/delete" {
        #[cfg(feature = "http")]
        {
            match net::chat_list().await {
                Ok(list) if list.is_empty() => { app.status = "no sessions".into(); push_toast(&mut app, "No sessions", ToastKind::Warn); }
                Ok(list) => { app.chat_modal = Some(ChatModal::Delete { list, sel: 0 }); app.status = "Select a session to delete (Enter)".into(); }
                Err(e) => { app.status = format!("list err: {}", e); push_toast(&mut app, format!("List error: {}", e), ToastKind::Error); }
            }
            return;
        }
    }
    if let Some(rest) = cmd.strip_prefix("/stream ") {
        let on = rest.trim().eq_ignore_ascii_case("on");
        let off = rest.trim().eq_ignore_ascii_case("off");
        if on || off {
            app.chat_stream = on;
            app.status = format!("streaming: {}", if on { "on" } else { "off" });
            return;
        }
        app.status = "usage: /stream on|off".into();
        return;
    }
    app.status = format!("unknown: {}", cmd);
}

async fn refresh_current(mut app: &mut App) {
    #[cfg(feature = "http")]
    {
        match app.active {
            Screen::Memory => {
                // Re-run search if we have a query
                if !app.mem_query.trim().is_empty() {
                    match net::memory_search(&app.mem_query, None, 25).await {
                        Ok(hits) => { app.mem_results = hits; app.mem_sel = 0; app.status = "search ok".into(); },
                        Err(e) => { app.status = format!("search err: {}", e); }
                    }
                }
                // Refresh pack preview
                match net::context_pack(None, 2048, 12, vec![]).await {
                    Ok(sum) => { app.mem_pack_summary = sum; push_toast(&mut app, "Memory pack refreshed", ToastKind::Success); },
                    Err(e) => { app.mem_pack_summary = format!("pack err: {}", e); push_toast(&mut app, format!("Pack error: {}", e), ToastKind::Error); }
                }
                // Keep digest fresh for details fallback
                if let Ok(d) = net::get_system_digest().await { app.sys_digest = d; }
            }
            Screen::Tools => {
                if let Ok(lst) = net::list_tools().await { app.tools_list = lst; }
                if let Ok(st) = net::list_tool_status().await { app.tools_status = st.into_iter().collect(); }
                app.status = "tools refreshed".into();
                push_toast(app, "Tools status refreshed", ToastKind::Success);
            }
            Screen::Codex => {
                if let Ok(cs) = net::codex_sessions().await { app.codex_sessions = cs; app.codex_sel = 0; }
                let detail = if let Some(s) = app.codex_sessions.get(0) { net::codex_session_detail(&s.session_id).await.ok() } else { None };
                app.codex_detail = detail.unwrap_or_else(|| "".into());
                push_toast(app, "Codex refreshed", ToastKind::Success);
            }
            Screen::Reports => {
                match net::list_reports().await {
                    Ok(list) => { app.reports = list; app.status = "reports refreshed".into(); push_toast(&mut app, "Reports refreshed", ToastKind::Success); },
                    Err(e) => { app.status = format!("reports err: {}", e); push_toast(&mut app, format!("Reports error: {}", e), ToastKind::Error); },
                }
            }
            Screen::Chat => {
                if let Ok((ok, ver)) = net::health().await { app.health_ok = ok; app.health_version = ver; }
                if let Ok(rows) = net::list_schedules().await { app.sched_rows = rows; }
                if let Ok(mtxt) = net::metrics_text().await { app.metrics_summary = net::summarize_metrics(&mtxt); }
                if let Ok(list) = net::list_reports().await { app.dash_reports = list; }
                push_toast(&mut app, "Dashboard refreshed", ToastKind::Success);
            }
            _ => {}
        }
    }
}

fn wrap_text_lines(text: &str, inner_width: u16) -> Vec<String> {
    if inner_width == 0 { return vec![String::new()]; }
    let w = inner_width as usize;
    let mut out: Vec<String> = Vec::new();
    for raw in text.split('\n') {
        let mut cur = String::new();
        let mut cur_w = 0usize;
        for ch in raw.chars() {
            let cw = UnicodeWidthChar::width(ch).unwrap_or(0);
            // If adding this char exceeds width, break line first
            if cur_w > 0 && cur_w + cw > w {
                out.push(cur);
                cur = String::new();
                cur_w = 0;
            }
            cur.push(ch);
            cur_w += cw;
            // If we exactly fill the width, also break line for next char
            if cur_w == w {
                out.push(cur);
                cur = String::new();
                cur_w = 0;
            }
        }
        out.push(cur);
    }
    if out.is_empty() { vec![String::new()] } else { out }
}

fn ui(f: &mut Frame, app: &App) {
    let size = f.size();
    // Fill full background with theme colors so unstyled areas are covered
    f.render_widget(Block::default().style(theme::body()), size);
    // Dynamic input height based on wrapped content lines
    let header_h: u16 = 3;
    let footer_h: u16 = 1;
    let reserved_body_min: u16 = 6; // keep some room for content
    let input_inner_w = size.width.saturating_sub(2); // borders
    let input_lines = wrap_text_lines(app.input.as_str(), input_inner_w);
    let mut input_outer_h = (input_lines.len() as u16).saturating_add(2); // borders
    let max_outer = size.height.saturating_sub(header_h + footer_h + reserved_body_min);
    if max_outer > 0 { input_outer_h = input_outer_h.min(max_outer); }
    if input_outer_h < 3 { input_outer_h = 3; } // at least one content line + borders

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(header_h), // header
            Constraint::Min(1),    // body
            Constraint::Length(input_outer_h), // input (dynamic)
            Constraint::Length(footer_h), // footer/status
        ])
        .split(size);

    // Header with its own boxed area && colored theme
    let header_block = Block::default()
        .borders(Borders::ALL)
        .title(" Foreman ")
        .title_alignment(Alignment::Left)
        .style(theme::header_block())
        .border_style(theme::header_border())
        .border_type(BorderType::Rounded);
    let header_area = chunks[0];
    let header_inner = header_block.inner(header_area);
    f.render_widget(header_block, header_area);

    // Tabs inside the header box, styled
    let titles = ["Chat", "Dashboard", "Tasks", "System", "Tools", "Codex", "Reports", "Research", "Settings"].iter().enumerate().map(|(i, t)| {
        let label = format!(" {} {} ", i + 1, t);
        Line::from(Span::styled(label, if app.active as usize == i { theme::tab_active() } else { theme::tab_inactive() }))
    });
    let tabs = Tabs::new(titles)
        .select(app.active as usize)
        .highlight_style(theme::tab_active())
        .style(Style::default().bg(theme::bg()).fg(theme::fg()));
    f.render_widget(tabs, header_inner);

    // Body area beneath header
    let inner = [chunks[1]];

    match app.active {
        Screen::Chat => screens::chat::draw(f, inner[0], app),
        Screen::Dashboard => screens::dashboard::draw(f, inner[0], app),
        Screen::Tasks => screens::tasks::draw(f, inner[0], app),
        Screen::Memory => screens::memory::draw(f, inner[0], app),
        Screen::Tools => screens::tools::draw(f, inner[0], app),
        Screen::Codex => screens::codex::draw(f, inner[0], app),
        Screen::Reports => screens::reports::draw(f, inner[0], app),
        Screen::Research => screens::research_arxiv::draw(f, inner[0], app),
        Screen::Settings => screens::settings::draw(f, inner[0], app),
    }

    // Input
    // Input border highlight: always focused on Chat and Codex, or on Memory search, or when editing params
    let input_focused = (app.active == Screen::Chat) || (app.active == Screen::Codex) || (app.active == Screen::Memory && app.focus_ix == 0) || app.editing_params;
    let input_border = if input_focused { theme::header_border() } else { Style::default().fg(theme::muted()) };
    let input_display: Vec<Line> = input_lines.into_iter().map(|s| Line::from(s)).collect();
    let mut input_title = String::from("Input");
    if app.voice_ptt { input_title.push_str(" [🎙]"); }
    else if app.rt_active { input_title.push_str(" [🎙 Live]"); }
    #[cfg(feature = "http")]
    if app.rt_active { input_title.push_str(" [🎙 Live]"); }
    let input = Paragraph::new(input_display)
        .block(Block::default().borders(Borders::ALL).title(input_title).border_style(input_border));
    f.render_widget(input, chunks[2]);

    // Status line
    let project_disp = app.proj_current.as_deref().map(|p| std::path::Path::new(p).file_name().and_then(|s| s.to_str()).unwrap_or(p)).unwrap_or("(none)");
    let hints = format!("Ctrl-1–8 tabs • Ctrl-Space talk • / commands • Ctrl-G projects • Ctrl-H help • Ctrl-R refresh • Ctrl-Q quit • Project: {}", project_disp);
    let status = Paragraph::new(format!("{}  —  {}", app.status, hints)).style(theme::status());
    f.render_widget(status, chunks[3]);

    // Toasts overlay (bottom-right), show recent, colored by kind
    let now = Instant::now();
    let mut active: Vec<&Toast> = app.toasts.iter().filter(|t| now.duration_since(t.at) < Duration::from_millis(3500)).collect();
    if !active.is_empty() {
        let max = active.len().min(3);
        active = active[active.len()-max..].to_vec();
        let width = 52u16;
        let height = (active.len() as u16) + 2;
        let area = Rect { x: size.x + size.width.saturating_sub(width + 2), y: size.y + size.height.saturating_sub(height + 2), width, height };
        let mut lines: Vec<Line> = vec![];
        for t in active {
            let style = match t.kind { ToastKind::Info => Style::default().fg(Color::Cyan), ToastKind::Success => Style::default().fg(Color::Green), ToastKind::Warn => Style::default().fg(Color::Yellow), ToastKind::Error => Style::default().fg(Color::Red) };
            lines.push(Line::from(t.msg.as_str()).style(style));
        }
        f.render_widget(Clear, area);
        let block = Block::default()
            .borders(Borders::ALL)
            .title("Notifications")
            .border_type(BorderType::Rounded)
            .border_style(theme::header_border())
            .style(Style::default().bg(theme::panel_bg()).fg(theme::fg()));
        let para = Paragraph::new(lines).block(block).style(Style::default().bg(theme::panel_bg()).fg(theme::fg()));
        f.render_widget(para, area);
    }

    // Help overlay
    if app.show_help {
        let area = centered_rect(70, 60, f.size());
        let lines = vec![
            "Navigation: F1–F7 or Alt-1–7 (Ctrl-1–7 if supported), Tab/Shift-Tab",
            "Commands: /..., Enter to run",
            "Refresh: Ctrl-R (Dashboard/System/Reports/Memory)",
            "Reports: ↑/↓ select, Enter/Ctrl-O open",
            "Memory: Ctrl-F focus search, Enter search, Ctrl-O open, Ctrl-P pin",
            "Tools: ←/→ servers, ↑/↓ tools, Ctrl-E edit params, Ctrl-L pick Steam game, Enter run",
            "Voice: Hold Ctrl-Space to dictate; Hang up: Ctrl-\\",
            "Realtime: /voice status | /voice end | /voice enable_wake | /voice disable_wake",
            "Codex: Ctrl-N new session, Ctrl-Y continue",
            "Projects: Ctrl-G picker, /proj scan | /proj pick PATH | /proj clear",
            "Logs: /logs [N] to view last N lines",
            "Selection: /select on|off to toggle mouse capture for copying",
            "Quit: Ctrl-Q  •  Help: Ctrl-H",
        ];
        let help_block = Block::default()
            .borders(Borders::ALL)
            .title("Help")
            .border_type(BorderType::Rounded)
            .border_style(theme::header_border())
            .style(Style::default().bg(theme::panel_bg()).fg(theme::fg()));
        let para = Paragraph::new(lines.join("\n")).block(help_block).style(Style::default().bg(theme::panel_bg()).fg(theme::fg()));
        f.render_widget(Clear, area);
        f.render_widget(para, area);
    }

    // Transient Ctrl hotkeys overlay (appears after pressing any Ctrl combo)
    if let Some(until) = app.ctrl_hints_until {
        if Instant::now() < until && !app.show_help {
            let width = 64u16;
            let height = 9u16;
            let area = Rect { x: size.x + 2, y: size.y + size.height.saturating_sub(height + 3), width, height };
            let lines = vec![
                "Ctrl hotkeys",
                "Tabs: Ctrl-1..8 • Help: Ctrl-H • Quit: Ctrl-Q",
                "Refresh: Ctrl-R • Open: Ctrl-O • Search: Ctrl-F • Pin: Ctrl-P",
                "Tools: Ctrl-E edit params • Voice: Hold Ctrl-Space",
                "Codex: Enter send • Ctrl-N new • Ctrl-Y continue",
            ];
            f.render_widget(Clear, area);
            let block = Block::default()
                .borders(Borders::ALL)
                .title("Hotkeys")
                .border_type(BorderType::Rounded)
                .border_style(theme::header_border())
                .style(Style::default().bg(theme::panel_bg()).fg(theme::fg()));
            let para = Paragraph::new(lines.join("\n")).block(block).wrap(Wrap { trim: true });
            f.render_widget(para, area);
        }
    }

    // Chat modal overlay (delete sessions)
    if let Some(ChatModal::Delete { list, sel }) = &app.chat_modal {
        let area = centered_rect(70, 60, f.size());
        let mut lines: Vec<Line> = vec![Line::from("Delete chat session (Enter to confirm, Esc to cancel)")];
        for (i, s) in list.iter().enumerate() {
            let title = s.title.clone().unwrap_or_else(|| "(no title)".into());
            let line = format!("{}: {}  {}", i + 1, &s.id[..8], title);
            let style = if *sel == i { theme::tab_active() } else { Style::default().fg(theme::fg()) };
            lines.push(Line::from(line).style(style));
        }
        let block = Block::default()
            .borders(Borders::ALL)
            .title("Chat Sessions")
            .border_type(BorderType::Rounded)
            .border_style(theme::header_border())
            .style(Style::default().bg(theme::panel_bg()).fg(theme::fg()));
        f.render_widget(Clear, area);
        let para = Paragraph::new(lines).block(block).wrap(Wrap { trim: false });
        f.render_widget(para, area);
    }

    // Project picker overlay
    if app.proj_modal_open {
        let area = centered_rect(70, 60, f.size());
        let mut lines: Vec<Line> = vec![Line::from("Pick a project (Enter, Esc)")];
        for (i, p) in app.proj_list.iter().enumerate() {
            let base = std::path::Path::new(p).file_name().and_then(|s| s.to_str()).unwrap_or(p);
            let line = format!("{}: {}", i + 1, base);
            let style = if app.proj_sel == i { theme::tab_active() } else { Style::default().fg(theme::fg()) };
            lines.push(Line::from(line).style(style));
        }
        f.render_widget(Clear, area);
        let block = Block::default()
            .borders(Borders::ALL)
            .title("Projects")
            .border_type(BorderType::Rounded)
            .border_style(theme::header_border())
            .style(Style::default().bg(theme::panel_bg()).fg(theme::fg()));
        let para = Paragraph::new(lines).block(block).wrap(Wrap { trim: true });
        f.render_widget(para, area);
    }

    // Steam games picker overlay
    if app.steam_modal_open {
        let area = centered_rect(70, 60, f.size());
        let mut lines: Vec<Line> = vec![Line::from("Pick a Steam game (Enter, Esc)")];
        for (i, (name, appid)) in app.steam_list.iter().enumerate() {
            let line = format!("{}: {} — {}", i + 1, name, appid);
            let style = if app.steam_sel == i { theme::tab_active() } else { Style::default().fg(theme::fg()) };
            lines.push(Line::from(line).style(style));
        }
        f.render_widget(Clear, area);
        let block = Block::default()
            .borders(Borders::ALL)
            .title("Steam Games")
            .border_type(BorderType::Rounded)
            .border_style(theme::header_border())
            .style(Style::default().bg(theme::panel_bg()).fg(theme::fg()));
        let para = Paragraph::new(lines).block(block).wrap(Wrap { trim: true });
        f.render_widget(para, area);
    }

    if let Some(p) = &app.ep_prompt {
        let area = centered_rect(70, 60, f.size());
        let mut text = vec![Line::from(p.title.as_str())];
        if let Some(cmds) = p.details.get("commands").and_then(|v| v.as_array()) {
            text.push(Line::from("Commands:"));
            for c in cmds.iter().filter_map(|x| x.as_str()) { text.push(Line::from(format!("  {}", c))); }
        }
        if let Some(ex) = &app.ep_explain { text.push(Line::from("")); text.push(Line::from(ex.as_str())); }
        let block = Block::default()
            .borders(Borders::ALL)
            .title("Approval Required")
            .border_type(BorderType::Rounded)
            .border_style(theme::header_border())
            .style(Style::default().bg(theme::panel_bg()).fg(theme::fg()));
        let para = Paragraph::new(text).block(block.clone()).wrap(Wrap { trim: true }).style(Style::default().bg(theme::panel_bg()).fg(theme::fg()));
        f.render_widget(Clear, area);
        f.render_widget(para, area);
        let btns = ["Yes","No","Explain This"];
        let lines: Vec<Line> = btns.iter().enumerate().map(|(i,b)| if i==app.ep_sel { Line::from(format!("[{}]", b)).style(Style::default().fg(Color::Cyan)) } else { Line::from(format!(" {} ", b)) }).collect();
        let btn_area = Rect { x: area.x + 2, y: area.y + area.height - 3, width: area.width - 4, height: 1 };
        let btn_block = Block::default().style(Style::default().bg(theme::panel_bg()).fg(theme::fg()));
        f.render_widget(Paragraph::new(lines).block(btn_block), btn_area);
    }
}

fn centered_rect(pct_x: u16, pct_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default().direction(Direction::Vertical).constraints([
        Constraint::Percentage((100 - pct_y) / 2), Constraint::Percentage(pct_y), Constraint::Percentage((100 - pct_y) / 2),]).split(r);
    Layout::default().direction(Direction::Horizontal).constraints([
        Constraint::Percentage((100 - pct_x) / 2), Constraint::Percentage(pct_x), Constraint::Percentage((100 - pct_x) / 2),]).split(popup_layout[1])[1]
}

pub(crate) fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max { s.to_string() } else { format!("{}…", &s[..max]) }
}

#[cfg(feature = "http")]
pub mod net {
    use serde::Deserialize;
    use futures_util::StreamExt;
    #[derive(Deserialize)]
    struct Digest { digest: String }

    pub async fn get_system_digest() -> anyhow::Result<String> {
        let d: Digest = reqwest::get("http://127.0.0.1:6061/api/system_map/digest").await?.json().await?;
        Ok(d.digest)
    }

    pub async fn system_map_refresh() -> anyhow::Result<()> {
        let client = reqwest::Client::new();
        let r = client.post("http://127.0.0.1:6061/api/system_map/refresh").send().await?;
        if r.status().is_success() { Ok(()) } else { anyhow::bail!("refresh failed: {}", r.status()) }
    }

    #[derive(Deserialize)]
    struct Task { id: i64, title: String }
    pub async fn list_tasks() -> anyhow::Result<Vec<String>> {
        let ts: Vec<Task> = reqwest::get("http://127.0.0.1:6061/api/tasks").await?.json().await?;
        Ok(ts.into_iter().map(|t| format!("{}: {}", t.id, t.title)).collect())
    }

    #[derive(Deserialize, Clone, Debug)]
    pub struct EphemeralPrompt { pub id: String, pub title: String, pub details: serde_json::Value }

    pub async fn get_approval_prompt() -> anyhow::Result<Option<EphemeralPrompt>> {
        let resp = reqwest::get("http://127.0.0.1:6061/api/approval/prompt").await?;
        if resp.status() == 204 { return Ok(None); }
        if resp.status().is_success() { Ok(Some(resp.json::<EphemeralPrompt>().await?)) } else { Ok(None) }
    }

    pub async fn answer_approval(id: &str, answer: &str) -> anyhow::Result<()> {
        let client = reqwest::Client::new();
        let _ = client.post("http://127.0.0.1:6061/api/approval/answer").json(&serde_json::json!({"id": id, "answer": answer})).send().await?;
        Ok(())
    }

    pub async fn explain_approval(id: &str) -> anyhow::Result<String> {
        let v: serde_json::Value = reqwest::get(&format!("http://127.0.0.1:6061/api/approval/explain/{}", id)).await?.json().await?;
        Ok(serde_json::to_string_pretty(&v).unwrap_or_else(|_| "(explain)".into()))
    }

    pub async fn audio_devices() -> anyhow::Result<String> {
        let resp = reqwest::get("http://127.0.0.1:6061/api/audio/devices").await?;
        let status = resp.status();
        let text = resp.text().await?;
        if !status.is_success() {
            anyhow::bail!(format!("/api/audio/devices http {}: {}", status, text));
        }
        Ok(text)
    }

    pub async fn audio_diagnose(transcribe: Option<bool>) -> anyhow::Result<String> {
        let req = serde_json::json!({
            "seconds": 6,
            "in_sr": 16000,
            "chunk_ms": 30,
            "sensitivity": 0.5,
            "min_speech_ms": 300,
            "transcribe": transcribe.unwrap_or(false),
        });
        let client = reqwest::Client::new();
        let resp = client.post("http://127.0.0.1:6061/api/audio/diagnose").json(&req).send().await?;
        let status = resp.status();
        let body = resp.text().await?;
        if !status.is_success() {
            if status.as_u16() == 404 {
                // Fallback: old core binary without audio endpoints
                let vt = voice_test_text().await.unwrap_or_else(|_| "(voice test fetch failed)".into());
                return Ok(format!("/api/audio/diagnose 404 (old core?) • /api/voice/test: {}", vt));
            }
            anyhow::bail!(format!("/api/audio/diagnose http {}: {}", status, body));
        }
        let v: serde_json::Value = serde_json::from_str(&body)?;
        // Summarize
        let opened = v.get("opened").and_then(|x| x.as_bool()).unwrap_or(false);
        let dev = v.get("input_device").and_then(|x| x.as_str()).unwrap_or("");
        let frames = v.get("frames").and_then(|x| x.as_u64()).unwrap_or(0);
        let segs = v.get("speech_segments").and_then(|x| x.as_u64()).unwrap_or(0);
        let avg = v.get("avg_energy").and_then(|x| x.as_f64()).unwrap_or(0.0);
        let peak = v.get("peak_energy").and_then(|x| x.as_f64()).unwrap_or(0.0);
        let thr = v.get("vad_threshold").and_then(|x| x.as_f64()).unwrap_or(0.0);
        let mut summary = format!("mic_open={} dev={} frames={} segs={} avg={:.3} peak={:.3} thr={:.3}", opened, dev, frames, segs, avg, peak, thr);
        if let Some(t) = v.get("transcript").and_then(|x| x.as_str()) { summary.push_str(&format!(" • transcript: {}", t)); }
        Ok(summary)
    }

    pub async fn voice_test_text() -> anyhow::Result<String> {
        let resp = reqwest::get("http://127.0.0.1:6061/api/voice/test").await?;
        let status = resp.status();
        let text = resp.text().await?;
        if status.is_success() { Ok(text) } else { anyhow::bail!(format!("/api/voice/test http {}: {}", status, text)) }
    }

    pub async fn audio_beep(args: Option<(u32, u32, f32)>) -> anyhow::Result<()> {
        let (seconds, out_sr, freq) = args.unwrap_or((1, 48000, 440.0));
        let req = serde_json::json!({"seconds": seconds, "out_sr": out_sr, "freq_hz": freq});
        let client = reqwest::Client::new();
        let resp = client.post("http://127.0.0.1:6061/api/audio/beep").json(&req).send().await?;
        if resp.status().is_success() { Ok(()) } else { anyhow::bail!(format!("/api/audio/beep http {}", resp.status())) }
    }

    // Realtime voice (status/stop)
    #[derive(Deserialize)]
    struct RtStatus { active: bool }
    pub async fn realtime_status() -> anyhow::Result<(bool, Option<String>)> {
        let v: serde_json::Value = reqwest::get("http://127.0.0.1:6061/api/realtime/status").await?.json().await?;
        let active = v.get("active").and_then(|b| b.as_bool()).unwrap_or(false);
        let err = v.get("last_error").and_then(|e| e.as_str()).map(|s| s.to_string());
        Ok((active, err))
    }
    pub async fn realtime_stop() -> anyhow::Result<()> {
        let _ = reqwest::Client::new().post("http://127.0.0.1:6061/api/realtime/stop").send().await?;
        Ok(())
    }
    #[derive(serde::Serialize)]
    struct RtStartBody {
        #[serde(skip_serializing_if = "Option::is_none")] model: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")] voice: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")] audio: Option<serde_json::Value>,
        #[serde(skip_serializing_if = "Option::is_none")] instructions: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")] endpoint: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")] transport: Option<String>,
    }
    pub async fn realtime_start(endpoint: Option<String>) -> anyhow::Result<()> {
        let body = RtStartBody {
            model: Some("gpt-realtime".into()),
            voice: Some("alloy".into()),
            audio: Some(serde_json::json!({"in_sr": 16000, "out_format": "pcm16"})),
            instructions: None,
            endpoint: endpoint.map(|ep| if ep.starts_with("ws://") || ep.starts_with("wss://") { ep } else { format!("ws://{}", ep) }),
            transport: None,
        };
        let resp = reqwest::Client::new().post("http://127.0.0.1:6061/api/realtime/start").json(&body).send().await?;
        if resp.status().is_success() { Ok(()) } else { anyhow::bail!(format!("realtime start http {}: {}", resp.status(), resp.text().await.unwrap_or_default())) }
    }
    // Wake endpoints
    pub async fn wake_status() -> anyhow::Result<serde_json::Value> {
        let v: serde_json::Value = reqwest::get("http://127.0.0.1:6061/api/wake/status").await?.json().await?;
        Ok(v)
    }
    pub async fn wake_daemon_status() -> anyhow::Result<serde_json::Value> {
        let v: serde_json::Value = reqwest::get("http://127.0.0.1:7071/v1/wake/health").await?.json().await?;
        Ok(v)
    }
    pub async fn wake_enable() -> anyhow::Result<()> {
        let _ = reqwest::Client::new().post("http://127.0.0.1:6061/api/wake/enable").send().await?;
        Ok(())
    }
    pub async fn wake_disable() -> anyhow::Result<()> {
        let _ = reqwest::Client::new().post("http://127.0.0.1:6061/api/wake/disable").send().await?;
        Ok(())
    }

    #[cfg(all(feature = "http", feature = "voice"))]
    pub async fn stt_transcribe_whisper(wav_bytes: Vec<u8>) -> anyhow::Result<String> {
        let key = std::env::var("OPENAI_API_KEY").map_err(|_| anyhow::anyhow!("OPENAI_API_KEY not set"))?;
        let part = reqwest::multipart::Part::bytes(wav_bytes).file_name("audio.wav").mime_str("audio/wav")?;
        let form = reqwest::multipart::Form::new()
            .text("model", std::env::var("OPENAI_STT_MODEL").unwrap_or_else(|_| "whisper-1".into()))
            .part("file", part);
        let client = reqwest::Client::new();
        let resp = client
            .post("https://api.openai.com/v1/audio/transcriptions")
            .bearer_auth(key)
            .multipart(form)
            .send()
            .await?;
        if !resp.status().is_success() { anyhow::bail!(format!("openai stt http {}", resp.status())); }
        let v: serde_json::Value = resp.json().await?;
        let text = v.get("text").and_then(|x| x.as_str()).unwrap_or("").to_string();
        Ok(text)
    }

    #[derive(Deserialize)]
    struct Approval { id: String, status: String }
    pub async fn list_approvals() -> anyhow::Result<Vec<String>> {
        let aps: Vec<Approval> = reqwest::get("http://127.0.0.1:6061/api/approvals").await?.json().await?;
        Ok(aps.into_iter().map(|a| format!("{}: {}", a.id, a.status)).collect())
    }

    #[derive(Deserialize)]
    struct ListDir { entries: Vec<String> }
    pub async fn tool_shell_list_dir(path: &str) -> anyhow::Result<Vec<String>> {
        let client = reqwest::Client::new();
        let resp = client.post("http://127.0.0.1:6061/api/tools/shell/list_dir").json(&serde_json::json!({"params": {"path": path}})).send().await?;
        let l: ListDir = resp.json().await?;
        Ok(l.entries)
    }

    // Dashboard
    #[derive(Deserialize)]
    struct Health { status: String, version: String }
    pub async fn health() -> anyhow::Result<(bool, String)> {
        let h: Health = reqwest::get("http://127.0.0.1:6061/health").await?.json().await?;
        Ok((h.status == "ok", h.version))
    }

    #[derive(Deserialize)]
    pub struct JobState { pub name: String, pub schedule: String, pub next_run: Option<String>, pub last_run: Option<String>, pub last_status: Option<String> }
    pub async fn list_schedules() -> anyhow::Result<Vec<String>> {
        let rows: Vec<JobState> = reqwest::get("http://127.0.0.1:6061/api/schedules").await?.json().await?;
        Ok(rows
            .into_iter()
            .map(|j| {
                let last = j.last_run.unwrap_or_else(|| "n/a".into());
                let status = j.last_status.unwrap_or_else(|| "?".into());
                format!("{} @ {}  |  next: {}  |  last: {} ({})", j.name, j.schedule, j.next_run.unwrap_or_else(|| "n/a".into()), last, status)
            })
            .collect())
    }

    pub async fn metrics_text() -> anyhow::Result<String> { Ok(reqwest::get("http://127.0.0.1:6061/metrics").await?.text().await?) }
    pub fn summarize_metrics(txt: &str) -> String {
        let mut api = 0u64;
        let mut jobs_ok = 0u64;
        let mut jobs_err = 0u64;
        for line in txt.lines() {
            if line.starts_with("foreman_api_requests_total") {
                if let Some(val) = line.split_whitespace().last().and_then(|v| v.parse::<u64>().ok()) {
                    api = api.saturating_add(val);
                }
            }
            if line.starts_with("foreman_scheduler_jobs_total") {
                if line.contains("status=\"ok\"") {
                    if let Some(val) = line.split_whitespace().last().and_then(|v| v.parse::<u64>().ok()) {
                        jobs_ok = jobs_ok.saturating_add(val);
                    }
                } else if line.contains("status=\"error\"") {
                    if let Some(val) = line.split_whitespace().last().and_then(|v| v.parse::<u64>().ok()) {
                        jobs_err = jobs_err.saturating_add(val);
                    }
                }
            }
        }
        format!("api={} • jobs ok={} err={}", api, jobs_ok, jobs_err)
    }

    // Tools inventory && calls
    pub async fn list_tools() -> anyhow::Result<Vec<(String, Vec<String>)>> {
        let v: Vec<(String, Vec<String>)> = reqwest::get("http://127.0.0.1:6061/api/tools").await?.json().await?;
        Ok(v)
    }
    pub async fn list_tool_status() -> anyhow::Result<Vec<(String, String)>> {
        let v: Vec<(String, String)> = reqwest::get("http://127.0.0.1:6061/api/tools/status").await?.json().await?;
        Ok(v)
    }
    pub async fn call_tool(server: &str, tool: &str, params_json: &str) -> anyhow::Result<String> {
        let params: serde_json::Value = serde_json::from_str(params_json).unwrap_or(serde_json::json!({}));
        let client = reqwest::Client::new();
        let resp = client
            .post(&format!("http://127.0.0.1:6061/api/tools/{}/{}", server, tool))
            .json(&serde_json::json!({"params": params}))
            .send()
            .await?;
        if !resp.status().is_success() { anyhow::bail!("tool failed: {}", resp.status()); }
        let v: serde_json::Value = resp.json().await?;
        Ok(serde_json::to_string_pretty(&v).unwrap_or_else(|_| "(result)".into()))
    }

    pub async fn run_job(job: &str) -> anyhow::Result<()> {
        let client = reqwest::Client::new();
        let resp = client.post(&format!("http://127.0.0.1:6061/api/schedules/run/{}", job)).send().await?;
        if !resp.status().is_success() { anyhow::bail!(format!("run job {} failed: {}", job, resp.status())); }
        Ok(())
    }

    // Reports (interim): browse storage/briefs via fs tools
    pub async fn list_reports() -> anyhow::Result<Vec<String>> {
        let client = reqwest::Client::new();
        let resp = client
            .post("http://127.0.0.1:6061/api/tools/fs/list")
            .json(&serde_json::json!({"params": {"path": "storage/briefs"}}))
            .send()
            .await?;
        if !resp.status().is_success() { anyhow::bail!("fs.list failed: {}", resp.status()); }
        let l: ListDir = resp.json().await?;
        let mut v = l.entries.into_iter().filter(|e| e.ends_with(".md")).collect::<Vec<_>>();
        v.sort();
        v.reverse();
        Ok(v)
    }

    #[derive(Deserialize)]
    struct ReadResp { content: String }
    pub async fn read_report(name: &str) -> anyhow::Result<String> {
        let path = format!("storage/briefs/{}", name);
        let client = reqwest::Client::new();
        let resp = client
            .post("http://127.0.0.1:6061/api/tools/fs/read")
            .json(&serde_json::json!({"params": {"path": path}}))
            .send()
            .await?;
        if !resp.status().is_success() { anyhow::bail!("fs.read failed: {}", resp.status()); }
        let r: ReadResp = resp.json().await?;
        Ok(r.content)
    }

    // OpenAI Chat (non-streaming)
    #[derive(Deserialize)]
    struct ChatChoice { message: ChatMessage }
    #[derive(Deserialize)]
    struct ChatMessage { role: String, content: String }
    #[derive(Deserialize)]
    struct ChatResp { choices: Vec<ChatChoice> }
    pub async fn chat_complete(history: &Vec<super::ChatMsg>) -> anyhow::Result<String> {
        let key = std::env::var("OPENAI_API_KEY").map_err(|_| anyhow::anyhow!("OPENAI_API_KEY not set"))?;
        let client = reqwest::Client::new();
        let msgs: Vec<serde_json::Value> = history.iter().map(|m| serde_json::json!({"role": m.role, "content": m.content})).collect();
        let body = serde_json::json!({
            "model": std::env::var("OPENAI_MODEL").unwrap_or_else(|_| "gpt-5".into()),
            "messages": msgs,
        });
        let resp = client.post("https://api.openai.com/v1/chat/completions")
            .bearer_auth(key)
            .header("content-type","application/json")
            .json(&body)
            .send().await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let txt = resp.text().await.unwrap_or_default();
            let snip = if txt.len() > 240 { format!("{}…", &txt[..240]) } else { txt };
            anyhow::bail!(format!("openai http {}: {}", status, snip));
        }
        let v: ChatResp = resp.json().await?;
        let reply = v.choices.get(0).map(|c| c.message.content.clone()).unwrap_or_else(|| "".into());
        Ok(reply)
    }

    pub async fn core_chat_complete(history: &Vec<super::ChatMsg>, model: &str) -> anyhow::Result<String> {
        let body = serde_json::json!({
            "messages": history.iter().map(|m| serde_json::json!({"role": m.role, "content": m.content})).collect::<Vec<_>>(),
            "model": model,
        });
        let resp = reqwest::Client::new().post("http://127.0.0.1:6061/api/chat/complete").json(&body).send().await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let txt = resp.text().await.unwrap_or_default();
            // Try parse ApiError { message }
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&txt) {
                if let Some(msg) = v.get("message").and_then(|s| s.as_str()) {
                    anyhow::bail!(format!("core chat {}: {}", status, msg));
                }
            }
            anyhow::bail!(format!("core chat http {}", status));
        }
        #[derive(Deserialize)]
        struct R { reply: String }
        let r: R = resp.json().await?;
        Ok(r.reply)
    }

    // Chat sessions persistence
    #[derive(Deserialize, Clone, Debug)]
    pub struct ChatSession { pub id: String, pub messages: Vec<super::ChatMsg> }
    #[derive(Deserialize, Clone, Debug)]
    pub struct SessionInfo { pub id: String, pub updated_at: String, pub title: Option<String> }

    pub async fn chat_latest() -> anyhow::Result<Option<ChatSession>> {
        let resp = reqwest::get("http://127.0.0.1:6061/api/chat/sessions/latest").await?;
        if resp.status() == reqwest::StatusCode::NO_CONTENT { return Ok(None); }
        if !resp.status().is_success() { anyhow::bail!("chat latest http {}", resp.status()); }
        Ok(Some(resp.json::<ChatSession>().await?))
    }

    pub async fn chat_new() -> anyhow::Result<String> {
        let resp = reqwest::Client::new().post("http://127.0.0.1:6061/api/chat/sessions").send().await?;
        if !resp.status().is_success() { anyhow::bail!("chat new http {}", resp.status()); }
        let s: ChatSession = resp.json().await?;
        Ok(s.id)
    }

    pub async fn chat_append(id: &str, role: &str, content: &str) -> anyhow::Result<()> {
        let body = serde_json::json!({"role": role, "content": content});
        let _ = reqwest::Client::new().post(&format!("http://127.0.0.1:6061/api/chat/sessions/{}/append", id)).json(&body).send().await?;
        Ok(())
    }

    pub async fn chat_list() -> anyhow::Result<Vec<SessionInfo>> {
        Ok(reqwest::get("http://127.0.0.1:6061/api/chat/sessions").await?.json().await?)
    }

    pub async fn chat_delete(id: &str) -> anyhow::Result<()> {
        let resp = reqwest::Client::new().delete(&format!("http://127.0.0.1:6061/api/chat/sessions/{}", id)).send().await?;
        if !resp.status().is_success() { anyhow::bail!("delete http {}", resp.status()); }
        Ok(())
    }

    // Memory search
    #[derive(Deserialize, Clone, Debug)]
    pub struct MemHit { pub atom_id: i64, pub snippet: String, pub score: f64 }
    #[derive(Deserialize, Clone, Debug)]
    pub struct MemAtom { pub id: i64, pub task_id: i64, pub kind: String, pub text: String, pub source: String, pub source_ref: Option<String>, pub importance: i64, pub pinned: bool, pub tokens_est: i64, pub parent_atom_id: Option<i64>, pub tags: Option<String>, pub hash: Option<String> }

    pub async fn memory_search(q: &str, task_id: Option<i64>, k: i64) -> anyhow::Result<Vec<MemHit>> {
        let mut url = reqwest::Url::parse("http://127.0.0.1:6061/api/memory/search")?;
        {
            let mut qp = url.query_pairs_mut();
            qp.append_pair("q", q);
            if let Some(t) = task_id { qp.append_pair("task_id", &t.to_string()); }
            qp.append_pair("k", &k.to_string());
        }
        let rows: Vec<MemHit> = reqwest::get(url).await?.json().await?;
        Ok(rows)
    }

    pub async fn get_atom(id: i64) -> anyhow::Result<MemAtom> {
        let v: MemAtom = reqwest::get(&format!("http://127.0.0.1:6061/api/memory/atoms/{}", id)).await?.json().await?;
        Ok(v)
    }

    pub async fn pin_atom(id: i64) -> anyhow::Result<()> {
        let client = reqwest::Client::new();
        let _ = client.post(&format!("http://127.0.0.1:6061/api/memory/atoms/{}/pin", id)).send().await?;
        Ok(())
    }
    pub async fn unpin_atom(id: i64) -> anyhow::Result<()> {
        let client = reqwest::Client::new();
        let _ = client.post(&format!("http://127.0.0.1:6061/api/memory/atoms/{}/unpin", id)).send().await?;
        Ok(())
    }

    // Pack preview summary helper
    #[derive(Deserialize)]
    struct PackCard { atom_id: i64, text: String, tokens_est: usize, importance: i32, pinned: bool }
    #[derive(Deserialize)]
    struct Pack { system_digest: String, task_digest: Option<String>, cards: Vec<PackCard>, expansions: Vec<String>, dropped: std::collections::HashMap<String, usize> }
    pub async fn context_pack(task_id: Option<i64>, token_budget: usize, k_cards: i64, expansions: Vec<String>) -> anyhow::Result<String> {
        let mut body = serde_json::json!({"token_budget": token_budget, "k_cards": k_cards, "expansions": expansions});
        if let Some(tid) = task_id { body["task_id"] = serde_json::json!(tid); }
        let client = reqwest::Client::new();
        let resp = client.post("http://127.0.0.1:6061/api/context/pack").json(&body).send().await?;
        if !resp.status().is_success() { anyhow::bail!("pack http {}", resp.status()); }
        let v: Pack = resp.json().await?;
        let total = v.cards.len();
        let pinned = v.cards.iter().filter(|c| c.pinned).count();
        let dropped = v.dropped.get("cards").cloned().unwrap_or(0);
        Ok(format!("cards={} pinned={} dropped={} expansions={} budget={}", total, pinned, dropped, v.expansions.len(), token_budget))
    }

    // Streaming chat: calls OpenAI with stream=true, invokes on_chunk for each delta,
    // and returns the final concatenated text.
    pub async fn chat_stream<F>(history: &Vec<super::ChatMsg>, mut on_chunk: F) -> anyhow::Result<String>
    where F: FnMut(String) + Send + 'static {
        let key = std::env::var("OPENAI_API_KEY").map_err(|_| anyhow::anyhow!("OPENAI_API_KEY not set"))?;
        let client = reqwest::Client::new();
        let msgs: Vec<serde_json::Value> = history.iter().map(|m| serde_json::json!({"role": m.role, "content": m.content})).collect();
        let body = serde_json::json!({
            "model": std::env::var("OPENAI_MODEL").unwrap_or_else(|_| "gpt-5".into()),
            "messages": msgs,
            "stream": true,
        });
        let resp = client.post("https://api.openai.com/v1/chat/completions")
            .bearer_auth(key)
            .header("content-type","application/json")
            .json(&body)
            .send().await?;
        if !resp.status().is_success() { anyhow::bail!(format!("openai http {}", resp.status())); }
        let mut acc = String::new();
        let mut buf = Vec::<u8>::new();
        let mut stream = resp.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let bytes = chunk?;
            buf.extend_from_slice(&bytes);
            // split by lines
            loop {
                if let Some(pos) = buf.iter().position(|&b| b == b'\n') {
                    let line = buf.drain(..=pos).collect::<Vec<u8>>();
                    let s = String::from_utf8_lossy(&line).to_string();
                    if let Some(rest) = s.strip_prefix("data: ") {
                        let data = rest.trim();
                        if data == "[DONE]" { return Ok(acc); }
                        if !data.is_empty() {
                            if let Ok(v) = serde_json::from_str::<serde_json::Value>(data) {
                                if let Some(choice) = v.get("choices").and_then(|c| c.as_array()).and_then(|a| a.get(0)) {
                                    if let Some(delta) = choice.get("delta") {
                                        if let Some(piece) = delta.get("content").and_then(|c| c.as_str()) {
                                            on_chunk(piece.to_string());
                                            acc.push_str(piece);
                                        }
                                    }
                                }
                            }
                        }
                    }
                } else { break; }
            }
        }
        Ok(acc)
    }

    // Core streaming with tools (SSE)
    pub async fn core_chat_stream<F>(history: &Vec<super::ChatMsg>, model: &str, mut on_event: F) -> anyhow::Result<String>
    where F: FnMut(String, String) + Send + 'static {
        let body = serde_json::json!({
            "messages": history.iter().map(|m| serde_json::json!({"role": m.role, "content": m.content})).collect::<Vec<_>>(),
            "model": model,
        });
        let client = reqwest::Client::new();
        let resp = client.post("http://127.0.0.1:6061/api/chat/stream").json(&body).send().await?;
        if !resp.status().is_success() { anyhow::bail!(format!("core stream http {}", resp.status())); }
        let mut acc = String::new();
        let mut cur_event = String::new();
        let mut cur_data = String::new();
        let mut stream = resp.bytes_stream();
        use futures_util::StreamExt;
        while let Some(chunk) = stream.next().await {
            let bytes = chunk?;
            for line in String::from_utf8_lossy(&bytes).split('\n') {
                let line = line.trim_end();
                if line.starts_with("event:") {
                    cur_event = line[6..].trim().to_string();
                } else if line.starts_with("data:") {
                    let d = line[5..].trim();
                    if !cur_data.is_empty() { cur_data.push_str("\n"); }
                    cur_data.push_str(d);
                    if cur_event == "token" { acc.push_str(d); }
                } else if line.is_empty() {
                    if !cur_event.is_empty() {
                        on_event(cur_event.clone(), cur_data.clone());
                    }
                    cur_event.clear();
                    cur_data.clear();
                }
            }
        }
        Ok(acc)
    }

    // --- Codex endpoints ---
    #[derive(Deserialize, Clone, Debug)]
    pub struct CodexSession { pub session_id: String, pub created_at: String }

    pub async fn codex_sessions() -> anyhow::Result<Vec<CodexSession>> {
        let v: Vec<CodexSession> = reqwest::get("http://127.0.0.1:6061/api/codex/sessions").await?.json().await?;
        Ok(v)
    }
    #[derive(Deserialize)]
    struct CodexDetail { text: String }
    pub async fn codex_session_detail(session_id: &str) -> anyhow::Result<String> {
        let d: CodexDetail = reqwest::get(&format!("http://127.0.0.1:6061/api/codex/session/{}", session_id)).await?.json().await?;
        Ok(d.text)
    }
    #[derive(Deserialize)]
    struct CodexNewResp { session_id: Option<String>, result: serde_json::Value }
    pub async fn codex_new(prompt: &str, repo: Option<&str>) -> anyhow::Result<String> {
        let body = serde_json::json!({"prompt": prompt, "repo": repo, "config": serde_json::json!({})});
        let resp = reqwest::Client::new().post("http://127.0.0.1:6061/api/codex/new").json(&body).send().await?;
        if !resp.status().is_success() { anyhow::bail!(format!("http {}", resp.status())); }
        let v: CodexNewResp = resp.json().await?;
        Ok(v.session_id.unwrap_or_else(|| "".into()))
    }
    pub async fn codex_continue(session_id: &str, prompt: &str) -> anyhow::Result<()> {
        let body = serde_json::json!({"session_id": session_id, "prompt": prompt});
        let resp = reqwest::Client::new().post("http://127.0.0.1:6061/api/codex/continue").json(&body).send().await?;
        if !resp.status().is_success() { anyhow::bail!(format!("http {}", resp.status())); }
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub enum ChatModal { Delete { list: Vec<net::SessionInfo>, sel: usize } }

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct ChatMsg { pub role: String, pub content: String }

#[derive(Clone, Debug)]
enum ChatEvent { AssistantReply(String), StreamDelta(String), StreamDone, Error(String), ToolNote(String), VoiceTranscript(String) }

// Re-export types for ease in UI code
pub use net::{MemHit, MemAtom};

#[inline]
fn current_memory_atom_id(app: &App) -> Option<i64> {
    if let Some(a) = app.mem_atom.as_ref() { Some(a.id) }
    else { app.mem_results.get(app.mem_sel).map(|h| h.atom_id) }
}

#[cfg(feature = "http")]
async fn open_memory_detail(app: &mut App) {
    if let Some(atom_id) = current_memory_atom_id(app) {
        match net::get_atom(atom_id).await {
            Ok(a) => { app.mem_atom = Some(a); app.status = format!("opened #{}", atom_id); push_toast(app, format!("Opened #{}", atom_id), ToastKind::Info); },
            Err(e) => { app.mem_atom = None; app.status = format!("open err: {}", e); push_toast(app, format!("Open error: {}", e), ToastKind::Error); }
        }
    }
}

#[cfg(feature = "http")]
async fn reload_memory_detail(app: &mut App, id: i64) {
    match net::get_atom(id).await { Ok(a) => app.mem_atom = Some(a), Err(_) => {} }
}

#[cfg(feature = "http")]
async fn reload_codex(app: &mut App) {
    if let Ok(cs) = net::codex_sessions().await { app.codex_sessions = cs; app.codex_sel = 0; }
    let detail = if let Some(s) = app.codex_sessions.get(0) { net::codex_session_detail(&s.session_id).await.ok() } else { None };
    app.codex_detail = detail.unwrap_or_default();
}

#[cfg(feature = "http")]
async fn reload_codex_detail(app: &mut App) {
    if let Some(s) = app.codex_sessions.get(app.codex_sel) {
        match net::codex_session_detail(&s.session_id).await { Ok(d) => app.codex_detail = d, Err(_) => {} }
    }
}
