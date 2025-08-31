#![cfg(feature = "tui")]
use ratatui::{prelude::*, widgets::*};
use crate::app::App;
use crate::theme;

pub fn draw(f: &mut Frame, area: Rect, app: &App) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(5), Constraint::Min(1)])
        .split(area);

    // Top: status summary
    let status = format!(
        "Server: {}  •  Version: {}  •  Approvals: {}  •  Metrics: {}",
        if app.health_ok { "OK" } else { "(down?)" },
        if app.health_version.is_empty() { "?".into() } else { app.health_version.clone() },
        app.approvals.len(),
        if app.metrics_summary.is_empty() { "n/a".into() } else { app.metrics_summary.clone() },
    );
    let sb = Paragraph::new(status)
        .block(Block::default().borders(Borders::ALL).title("Status").border_style(theme::header_border()))
        .style(theme::body());
    f.render_widget(sb, rows[0]);

    // Bottom: schedules and recent briefs
    let bottom = Layout::default().direction(Direction::Horizontal).constraints([Constraint::Percentage(60), Constraint::Percentage(40)]).split(rows[1]);

    let list = if app.sched_rows.is_empty() { vec![ListItem::new("(no schedules)")] } else { app.sched_rows.iter().map(|s| ListItem::new(s.as_str())).collect() };
    let lb = List::new(list).block(Block::default().borders(Borders::ALL).title("Schedules").border_style(theme::header_border()));
    f.render_widget(lb, bottom[0]);

    let mut briefs = app.dash_reports.clone();
    briefs.sort(); briefs.reverse();
    briefs.truncate(3);
    let ritems: Vec<ListItem> = if briefs.is_empty() { vec![ListItem::new("(no briefs)")] } else { briefs.iter().map(|b| ListItem::new(b.as_str())).collect() };
    let rb = List::new(ritems).block(Block::default().borders(Borders::ALL).title("Recent Briefs").border_style(theme::header_border()));
    f.render_widget(rb, bottom[1]);
}
