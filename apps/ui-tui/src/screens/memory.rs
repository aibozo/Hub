#![cfg(feature = "tui")]
use ratatui::{prelude::*, widgets::*};
use crate::app::App;
use crate::theme;

pub fn draw(f: &mut Frame, area: Rect, app: &App) {
    // Columns: left (search + results), right (details + pack)
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(45), Constraint::Percentage(55)])
        .split(area);

    // Left side split: search (3 lines) + results
    let left = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(1)])
        .split(cols[0]);

    // Search panel
    let search_active = app.active == crate::app::Screen::Memory && app.focus_ix == 0;
    let search_title = if search_active { "Search (Ctrl-F) *" } else { "Search (Ctrl-F)" };
    let q = if search_active { app.input.as_str() } else { app.mem_query.as_str() };
    let search = Paragraph::new(q)
        .block(Block::default().borders(Borders::ALL).title(search_title).border_type(BorderType::Rounded).border_style(theme::header_border()))
        .style(Style::default().bg(theme::panel_bg()).fg(theme::fg()));
    f.render_widget(search, left[0]);

    // Results list
    let items: Vec<ListItem> = app
        .mem_results
        .iter()
        .enumerate()
        .map(|(i, h)| {
            let line = format!("#{:<6}  {:>6.2}  {}", h.atom_id, h.score, h.snippet.replace('\n', " "));
            let style = if i == app.mem_sel { theme::tab_active() } else { Style::default().fg(theme::fg()) };
            ListItem::new(Line::from(line).style(style))
        })
        .collect();
    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Results (Enter to open, Ctrl-P pin)").border_style(theme::header_border()))
        .highlight_style(theme::tab_active());
    f.render_widget(list, left[1]);

    // Right side split: details + pack preview
    let right = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(6), Constraint::Length(8)])
        .split(cols[1]);

    // Details
    let mut lines: Vec<Line> = vec![];
    if let Some(a) = &app.mem_atom {
        lines.push(Line::from(format!("id={}  pinned={}  imp={}  src={}", a.id, a.pinned, a.importance, a.source)));
        if let Some(sr) = a.source_ref.as_ref() { lines.push(Line::from(format!("ref: {}", sr))); }
        lines.push(Line::from(""));
        lines.push(Line::from(a.text.as_str()));
    } else {
        let digest = if app.sys_digest.is_empty() { "(no system digest)".into() } else { app.sys_digest.clone() };
        lines.push(Line::from("System Digest:"));
        lines.push(Line::from(digest));
    }
    let detail = Paragraph::new(lines)
        .wrap(Wrap { trim: true })
        .block(Block::default().borders(Borders::ALL).title("Details").border_style(theme::header_border()));
    f.render_widget(detail, right[0]);

    // Pack preview summary
    let pack = Paragraph::new(if app.mem_pack_summary.is_empty() { "(no pack)" } else { app.mem_pack_summary.as_str() })
        .block(Block::default().borders(Borders::ALL).title("Context Pack (Ctrl-R refresh)").border_style(theme::header_border()))
        .style(Style::default().bg(theme::panel_bg()).fg(theme::fg()));
    f.render_widget(pack, right[1]);
}
