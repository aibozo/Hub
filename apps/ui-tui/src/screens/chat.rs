#![cfg(feature = "tui")]
use ratatui::{prelude::*, widgets::*};
use crate::app::{App, Screen};
use crate::theme;

pub fn draw(f: &mut Frame, area: Rect, app: &App) {
    // Chat messages window (input is global at bottom)
    let title = "Chat";
    let mut lines: Vec<Line<'_>> = Vec::new();
    if app.chat_messages.is_empty() {
        lines.push(Line::from("(Type a message and press Enter)"));
    } else {
        for m in &app.chat_messages {
            let who = if m.role == "assistant" { "AI" } else { "You" };
            let style = if m.role == "assistant" { Style::default().fg(Color::Cyan) } else { Style::default().fg(Color::Green) };
            lines.push(Line::from(format!("{}:", who)).style(style));
            for l in m.content.lines() { lines.push(Line::from(format!("  {}", l))); }
            lines.push(Line::from(""));
        }
    }
    let block = Block::default().borders(Borders::ALL).title(title).border_style(theme::header_border());
    // Show bottom of the feed with optional scrollback
    let max_visible = area.height.saturating_sub(2) as usize; // borders
    let total = lines.len();
    let max_scroll = total.saturating_sub(max_visible);
    let offset = app.chat_scroll.min(max_scroll);
    let start = total.saturating_sub(max_visible + offset);
    let p = Paragraph::new(lines.into_iter().skip(start).collect::<Vec<_>>())
        .block(block)
        .wrap(Wrap { trim: true });
    f.render_widget(p, area);
}
