#![cfg(feature = "tui")]
use ratatui::{prelude::*, widgets::*};
use crate::app::App;

pub fn draw(f: &mut Frame, area: Rect, app: &App) {
    let items: Vec<ListItem> = app.tasks.iter().map(|t| ListItem::new(t.as_str())).collect();
    let list = List::new(items).block(Block::default().borders(Borders::ALL).title("Tasks"));
    f.render_widget(list, area);
}

