#![cfg(feature = "tui")]
use ratatui::{prelude::*, widgets::*};
use crate::app::App;
use crate::theme;

pub fn draw(f: &mut Frame, area: Rect, app: &App) {
    // Layout: [Sessions] | [Detail]
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
        .split(area);

    // Left: sessions list
    let items: Vec<ListItem> = if app.codex_sessions.is_empty() {
        vec![ListItem::new("(no Codex sessions)")]
    } else {
        app.codex_sessions.iter().map(|s| {
            let short = if s.session_id.len() > 8 { &s.session_id[..8] } else { &s.session_id };
            let label = format!("{}  {}", short, s.created_at);
            ListItem::new(label)
        }).collect()
    };
    let mut sstate = ListState::default();
    if !app.codex_sessions.is_empty() { sstate.select(Some(app.codex_sel.min(app.codex_sessions.len().saturating_sub(1)))); }
    let sblock_border = if app.active == crate::app::Screen::Codex && app.focus_ix == 0 { theme::header_border() } else { Style::default().fg(theme::muted()) };
    let sblock = Block::default().borders(Borders::ALL).title("Codex Sessions (Ctrl-N: new, Ctrl-Y: continue)").border_style(sblock_border);
    let slist = List::new(items).highlight_style(theme::tab_active());
    f.render_stateful_widget(slist.block(sblock), cols[0], &mut sstate);

    // Right: detail area
    let border = if app.active == crate::app::Screen::Codex && app.focus_ix == 1 { theme::header_border() } else { Style::default().fg(theme::muted()) };
    let text = if app.codex_detail.is_empty() { "(select a session or press n to start)" } else { app.codex_detail.as_str() };
    let para = Paragraph::new(text).wrap(Wrap { trim: false }).block(Block::default().borders(Borders::ALL).title("Conversation").border_style(border));
    f.render_widget(para, cols[1]);
}
