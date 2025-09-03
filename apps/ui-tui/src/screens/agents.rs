#![cfg(feature = "tui")]
use ratatui::{prelude::*, widgets::*};
use crate::app::App;
use crate::theme;

pub fn draw(f: &mut Frame, area: Rect, app: &App) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
        .split(area);

    // Left: agents list
    let items: Vec<ListItem> = if app.agents_rows.is_empty() {
        vec![ListItem::new("(no agents)")]
    } else {
        app.agents_rows.iter().map(|a| {
            let short = if a.id.len() > 8 { &a.id[..8] } else { &a.id };
            let label = format!("{}  [{}] {}", short, a.status, a.title);
            ListItem::new(label)
        }).collect()
    };
    let mut sstate = ListState::default();
    if !app.agents_rows.is_empty() { sstate.select(Some(app.agents_sel.min(app.agents_rows.len().saturating_sub(1)))); }
    let sborder = if app.active == crate::app::Screen::Agents && app.focus_ix == 0 { theme::header_border() } else { Style::default().fg(theme::muted()) };
    let slist = List::new(items).block(Block::default().borders(Borders::ALL).title("Agents").border_style(sborder)).highlight_style(theme::tab_active());
    f.render_stateful_widget(slist, cols[0], &mut sstate);

    // Right: detail
    let border = if app.active == crate::app::Screen::Agents && app.focus_ix == 1 { theme::header_border() } else { Style::default().fg(theme::muted()) };
    let text = if app.agents_detail.is_empty() { "(select an agent)" } else { app.agents_detail.as_str() };
    let para = Paragraph::new(text).wrap(Wrap { trim: false }).block(Block::default().borders(Borders::ALL).title("Runlog").border_style(border));
    f.render_widget(para, cols[1]);
}

