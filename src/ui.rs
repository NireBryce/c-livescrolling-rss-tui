//! Terminal UI rendering.
//!
//! All drawing logic lives here, separated from application state ([`App`])
//! and input handling ([`crate::input`]).  This makes it easy to change the
//! visual layout without touching business logic.
//!
//! ## For contributors
//!
//! * The layout is a two-row split: a scrollable list on top and a one-line
//!   status bar at the bottom.
//! * Colours and styles are defined inline — feel free to extract them into
//!   constants or a theme struct if the palette grows.
//! * [`ratatui`] is the TUI framework; see its docs for widget details.

use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use crate::app::App;

/// Draw the complete UI for one frame.
///
/// Called once per tick from the main loop.  Delegates to helper functions
/// for each screen region.
pub fn draw(app: &mut App, frame: &mut Frame) {
    let [main_area, status_area] = Layout::vertical([
        Constraint::Min(1),
        Constraint::Length(1),
    ])
    .areas(frame.area());

    draw_feed_list(app, frame, main_area);
    draw_status_bar(app, frame, status_area);
}

/// Render the scrollable feed item list.
fn draw_feed_list(app: &mut App, frame: &mut Frame, area: Rect) {
    let list_items: Vec<ListItem> = app
        .items
        .iter()
        .map(|item| {
            let date_str = item
                .published
                .map(|d| d.format("%Y-%m-%d %H:%M").to_string())
                .unwrap_or_else(|| "no date".into());

            let line = Line::from(vec![
                Span::styled(
                    format!("{:<18}", date_str),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::raw(" "),
                Span::styled(&item.title, Style::default().fg(Color::White)),
                Span::raw("  "),
                Span::styled(
                    format!("[{}]", item.source_name),
                    Style::default().fg(Color::Cyan),
                ),
            ]);

            ListItem::new(line)
        })
        .collect();

    let list = List::new(list_items)
        .block(
            Block::default()
                .title(" RSS Feed ")
                .borders(Borders::ALL),
        )
        .highlight_style(
            Style::default()
                .add_modifier(Modifier::BOLD)
                .bg(Color::DarkGray),
        )
        .highlight_symbol("▸ ");

    frame.render_stateful_widget(list, area, &mut app.list_state);
}

/// Render the bottom status bar.
fn draw_status_bar(app: &App, frame: &mut Frame, area: Rect) {
    let status = Paragraph::new(Line::from(vec![
        Span::styled(" ", Style::default()),
        Span::styled(&app.status, Style::default().fg(Color::Yellow)),
        Span::raw("  "),
        Span::styled(
            format!("{} items", app.items.len()),
            Style::default().fg(Color::Green),
        ),
        Span::raw("  q: quit  ↑/↓: scroll  Home/End: jump"),
    ]));
    frame.render_widget(status, area);
}
