#![cfg(feature = "tui")]
use ratatui::{prelude::*, widgets::*};
use crate::app::App;

pub fn draw(f: &mut Frame, area: Rect, app: &App) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(25),
            Constraint::Percentage(45),
            Constraint::Percentage(30),
        ])
        .split(area);

    // Left: Saved topics / actions (placeholder list)
    let left_border = if app.active == crate::app::Screen::Research && app.focus_ix == 0 { Style::default().fg(Color::Yellow) } else { Style::default().fg(Color::Gray) };
    let left_items: Vec<ListItem> = vec![
        ListItem::new("Topics (soon):"),
        ListItem::new("- recent cs.* (3d)"),
        ListItem::new("- LLM quantization (7d)"),
        ListItem::new("- MoE routing (7d)"),
        ListItem::new("")
    ];
    let mut left_state = ListState::default();
    let left = List::new(left_items)
        .block(Block::default().borders(Borders::ALL).title("Research — arXiv").border_style(left_border))
        .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));
    f.render_stateful_widget(left, cols[0], &mut left_state);

    // Center: Results list
    let center_border = if app.active == crate::app::Screen::Research && app.focus_ix == 1 { Style::default().fg(Color::Yellow) } else { Style::default().fg(Color::Gray) };
    let items: Vec<ListItem> = if app.research_results.is_empty() {
        vec![ListItem::new("(Ctrl-R to search using the input box) ")]
    } else {
        app.research_results.iter().map(|s| ListItem::new(s.as_str())).collect()
    };
    let mut state = ListState::default();
    if !app.research_results.is_empty() { state.select(Some(app.research_sel.min(app.research_results.len().saturating_sub(1)))); }
    let center = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Results").border_style(center_border))
        .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));
    f.render_stateful_widget(center, cols[1], &mut state);

    // Right: Details pane
    let right_border = if app.active == crate::app::Screen::Research && app.focus_ix == 2 { Style::default().fg(Color::Yellow) } else { Style::default().fg(Color::Gray) };
    let details = if app.research_details.is_empty() {
        "(Select a paper and press Enter to summarize. 'd' downloads PDF; 'b' runs brief.)".to_string()
    } else { app.research_details.clone() };
    let para = Paragraph::new(details).wrap(Wrap { trim: false }).block(Block::default().borders(Borders::ALL).title("Details").border_style(right_border));
    f.render_widget(para, cols[2]);
}

