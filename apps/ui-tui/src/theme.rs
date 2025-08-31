#![cfg(feature = "tui")]
use ratatui::style::{Color, Modifier, Style};
use std::sync::atomic::{AtomicU8, Ordering};

#[derive(Copy, Clone)]
pub enum Mode { AmberDark = 0, Default = 1 }

static MODE: AtomicU8 = AtomicU8::new(Mode::AmberDark as u8);

pub fn set_mode(m: Mode) { MODE.store(m as u8, Ordering::Relaxed); }
pub fn toggle_mode() {
    let m = MODE.load(Ordering::Relaxed);
    let next = if m == Mode::AmberDark as u8 { Mode::Default } else { Mode::AmberDark };
    set_mode(next);
}
fn mode() -> Mode { if MODE.load(Ordering::Relaxed) == Mode::AmberDark as u8 { Mode::AmberDark } else { Mode::Default } }
pub fn mode_name() -> &'static str { match mode() { Mode::AmberDark => "AmberDark", Mode::Default => "Default" } }

// Palette
pub fn bg() -> Color { match mode() { Mode::AmberDark => Color::Rgb(10,10,10), Mode::Default => Color::Black } }
pub fn fg() -> Color { match mode() { Mode::AmberDark => Color::Rgb(220,220,220), Mode::Default => Color::White } }
pub fn accent() -> Color { match mode() { Mode::AmberDark => Color::Rgb(255,179,0), Mode::Default => Color::Cyan } }
pub fn muted() -> Color { match mode() { Mode::AmberDark => Color::Rgb(150,150,150), Mode::Default => Color::Gray } }

// Common styles
pub fn header_block() -> Style { Style::default().bg(bg()) }
pub fn header_border() -> Style { Style::default().fg(accent()) }
pub fn tab_active() -> Style { Style::default().fg(accent()).add_modifier(Modifier::BOLD) }
pub fn tab_inactive() -> Style { Style::default().fg(muted()) }
pub fn body() -> Style { Style::default().bg(bg()).fg(fg()) }
pub fn status() -> Style { Style::default().fg(muted()) }
pub fn status_ok() -> Style { Style::default().fg(Color::Green) }
pub fn status_warn() -> Style { Style::default().fg(Color::Yellow) }
pub fn status_err() -> Style { Style::default().fg(Color::Red) }

// Panels (e.g., notifications) use a slightly lighter bg for contrast
pub fn panel_bg() -> Color {
    match mode() {
        Mode::AmberDark => Color::Rgb(22, 22, 22),
        Mode::Default => Color::Rgb(30, 30, 30),
    }
}
