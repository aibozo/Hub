#![cfg(feature = "tui")]
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

// Centralized hotkey map to avoid drift.
// Only Ctrl-combos (plus function keys) to avoid stealing plain typing.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hotkey {
    None,
    Quit,
    SwitchTab(usize), // 0-based index
    ToggleHelp,
    Refresh,
    OpenReport,
    EditToolParams,
    FocusSearch,
    TogglePin,
    CodexNew,
    CodexContinue,
    OpenProjectPicker,
    PickSteamGame,
    VoicePTT, // hold Ctrl+Space to record
    VoiceHangup, // Ctrl-\ to end realtime call
}

pub fn resolve(ev: KeyEvent) -> Hotkey {
    let m = ev.modifiers;
    match (m, ev.code) {
        (KeyModifiers::CONTROL, KeyCode::Char('c')) | (KeyModifiers::CONTROL, KeyCode::Char('q')) => Hotkey::Quit,
        (KeyModifiers::NONE, KeyCode::F(1)) | (KeyModifiers::CONTROL, KeyCode::Char('1')) => Hotkey::SwitchTab(0),
        (KeyModifiers::NONE, KeyCode::F(2)) | (KeyModifiers::CONTROL, KeyCode::Char('2')) => Hotkey::SwitchTab(1),
        (KeyModifiers::NONE, KeyCode::F(3)) | (KeyModifiers::CONTROL, KeyCode::Char('3')) => Hotkey::SwitchTab(2),
        (KeyModifiers::NONE, KeyCode::F(4)) | (KeyModifiers::CONTROL, KeyCode::Char('4')) => Hotkey::SwitchTab(3),
        (KeyModifiers::NONE, KeyCode::F(5)) | (KeyModifiers::CONTROL, KeyCode::Char('5')) => Hotkey::SwitchTab(4),
        (KeyModifiers::NONE, KeyCode::F(6)) | (KeyModifiers::CONTROL, KeyCode::Char('6')) => Hotkey::SwitchTab(5),
        (KeyModifiers::NONE, KeyCode::F(7)) | (KeyModifiers::CONTROL, KeyCode::Char('7')) => Hotkey::SwitchTab(6),
        (KeyModifiers::NONE, KeyCode::F(8)) | (KeyModifiers::CONTROL, KeyCode::Char('8')) => Hotkey::SwitchTab(7),
        // Some terminals don't send Ctrl with digits; add Alt as a reliable alternative
        (KeyModifiers::ALT, KeyCode::Char('1')) => Hotkey::SwitchTab(0),
        (KeyModifiers::ALT, KeyCode::Char('2')) => Hotkey::SwitchTab(1),
        (KeyModifiers::ALT, KeyCode::Char('3')) => Hotkey::SwitchTab(2),
        (KeyModifiers::ALT, KeyCode::Char('4')) => Hotkey::SwitchTab(3),
        (KeyModifiers::ALT, KeyCode::Char('5')) => Hotkey::SwitchTab(4),
        (KeyModifiers::ALT, KeyCode::Char('6')) => Hotkey::SwitchTab(5),
        (KeyModifiers::ALT, KeyCode::Char('7')) => Hotkey::SwitchTab(6),
        (KeyModifiers::ALT, KeyCode::Char('8')) => Hotkey::SwitchTab(7),
        (KeyModifiers::CONTROL, KeyCode::Char('h')) => Hotkey::ToggleHelp,
        (KeyModifiers::CONTROL, KeyCode::Char('r')) => Hotkey::Refresh,
        (KeyModifiers::CONTROL, KeyCode::Char('o')) => Hotkey::OpenReport,
        (KeyModifiers::CONTROL, KeyCode::Char('e')) => Hotkey::EditToolParams,
        (KeyModifiers::CONTROL, KeyCode::Char('f')) => Hotkey::FocusSearch,
        (KeyModifiers::CONTROL, KeyCode::Char('p')) => Hotkey::TogglePin,
        (KeyModifiers::CONTROL, KeyCode::Char('n')) => Hotkey::CodexNew,
        // Avoid Ctrl-C (Quit); use Ctrl-Y for Codex continue
        (KeyModifiers::CONTROL, KeyCode::Char('y')) => Hotkey::CodexContinue,
        (KeyModifiers::CONTROL, KeyCode::Char('g')) => Hotkey::OpenProjectPicker,
        (KeyModifiers::CONTROL, KeyCode::Char('l')) => Hotkey::PickSteamGame,
        (KeyModifiers::CONTROL, KeyCode::Char(' ')) => Hotkey::VoicePTT,
        (KeyModifiers::CONTROL, KeyCode::Char('\\')) => Hotkey::VoiceHangup,
        _ => Hotkey::None,
    }
}
