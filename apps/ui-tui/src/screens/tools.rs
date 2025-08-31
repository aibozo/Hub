#![cfg(feature = "tui")]
use ratatui::{prelude::*, widgets::*};
use crate::app::App;
use crate::theme;

pub fn draw(f: &mut Frame, area: Rect, app: &App) {
    // Layout: [Servers] | [Tools + Params / Output]
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(area);

    // Left: servers list
    let server_items: Vec<ListItem> = if app.tools_list.is_empty() {
        vec![ListItem::new("(no servers)")]
    } else {
        app.tools_list.iter().map(|(s, _)| {
            let status = app.tools_status.get(s).cloned().unwrap_or_else(|| "?".into());
            let label = format!("{}  [{}]", s, status);
            ListItem::new(label)
        }).collect()
    };
    let mut sstate = ListState::default();
    if !app.tools_list.is_empty() { sstate.select(Some(app.tool_server_sel.min(app.tools_list.len().saturating_sub(1)))); }
    let sblock_border = if app.active == crate::app::Screen::Tools && app.focus_ix == 0 { theme::header_border() } else { Style::default().fg(theme::muted()) };
    let sblock = Block::default().borders(Borders::ALL).title("Servers").border_style(sblock_border);
    let slist = List::new(server_items).highlight_style(theme::tab_active());
    f.render_stateful_widget(slist.block(sblock), cols[0], &mut sstate);

    let right = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(35), Constraint::Length(3), Constraint::Percentage(62)])
        .split(cols[1]);

    // Top-right: tools of selected server
    let tools = app.tools_list.get(app.tool_server_sel).map(|(_, t)| t).cloned().unwrap_or_default();
    let tool_items: Vec<ListItem> = if tools.is_empty() { vec![ListItem::new("(no tools)")] } else { tools.iter().map(|t| ListItem::new(t.as_str())).collect() };
    let mut tstate = ListState::default();
    if !tools.is_empty() { tstate.select(Some(app.tool_tool_sel.min(tools.len().saturating_sub(1)))); }
    let tblock_border = if app.active == crate::app::Screen::Tools && app.focus_ix == 1 { theme::header_border() } else { Style::default().fg(theme::muted()) };
    let tblock = Block::default().borders(Borders::ALL).title("Tools").border_style(tblock_border);
    let tlist = List::new(tool_items).highlight_style(theme::tab_active());
    f.render_stateful_widget(tlist.block(tblock), right[0], &mut tstate);

    // Middle: params editor (single-line display; edited via 'e')
    let ptitle = if app.editing_params { "Params (editing: Enter to save)" } else { "Params (Ctrl-E to edit JSON)" };
    let pborder = if app.active == crate::app::Screen::Tools && app.focus_ix == 2 || app.editing_params { theme::header_border() } else { Style::default().fg(theme::muted()) };
    let p = Paragraph::new(app.tool_params.as_str()).block(Block::default().borders(Borders::ALL).title(ptitle).border_style(pborder));
    f.render_widget(p, right[1]);

    // Bottom-right: output viewer
    let out = if app.tool_output_text.is_empty() { "(run selected tool with Enter)" } else { app.tool_output_text.as_str() };
    let oborder = if app.active == crate::app::Screen::Tools && app.focus_ix == 3 { theme::header_border() } else { Style::default().fg(theme::muted()) };
    let o = Paragraph::new(out).wrap(Wrap { trim: false }).block(Block::default().borders(Borders::ALL).title("Output").border_style(oborder));
    f.render_widget(o, right[2]);
}
