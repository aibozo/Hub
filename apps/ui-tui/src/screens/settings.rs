#![cfg(feature = "tui")]
use ratatui::{prelude::*, widgets::*};
use crate::app::{App, truncate};

pub fn draw(f: &mut Frame, area: Rect, app: &App) {
    let text = if app.sys_digest.is_empty() { "(no digest)".to_string() } else { truncate(&app.sys_digest, 200) };
    let p = Paragraph::new(text).block(Block::default().borders(Borders::ALL).title("System Map Digest"));
    f.render_widget(p, area);
}

