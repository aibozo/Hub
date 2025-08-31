#![cfg(feature = "tui")]
use ratatui::{prelude::*, widgets::*};
use crate::app::App;

pub fn draw(f: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(area);

    // Left: list of reports (briefs)
    let items: Vec<ListItem> = if app.reports.is_empty() {
        vec![ListItem::new("(no reports)")]
    } else {
        app.reports.iter().map(|s| ListItem::new(s.as_str())).collect()
    };
    let lborder = if app.active == crate::app::Screen::Reports && app.focus_ix == 0 { Style::default().fg(ratatui::style::Color::Yellow) } else { Style::default().fg(ratatui::style::Color::Gray) };
    let list = List::new(items)
        .highlight_style(Style::default().fg(ratatui::style::Color::Yellow).add_modifier(Modifier::BOLD))
        .block(Block::default().borders(Borders::ALL).title("Reports (storage/briefs)").border_style(lborder));
    let mut state = ratatui::widgets::ListState::default();
    if !app.reports.is_empty() { state.select(Some(app.report_sel.min(app.reports.len().saturating_sub(1)))); }
    f.render_stateful_widget(list, chunks[0], &mut state);

    // Right: preview
    let preview = if app.report_content.is_empty() { "(select a report and press Enter)".to_string() } else { app.report_content.clone() };
    let pborder = if app.active == crate::app::Screen::Reports && app.focus_ix == 1 { Style::default().fg(ratatui::style::Color::Yellow) } else { Style::default().fg(ratatui::style::Color::Gray) };
    let para = Paragraph::new(preview).wrap(Wrap { trim: false }).block(Block::default().borders(Borders::ALL).title("Preview").border_style(pborder));
    f.render_widget(para, chunks[1]);
}

