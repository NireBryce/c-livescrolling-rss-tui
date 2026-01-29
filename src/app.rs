use std::collections::HashMap;

use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::source::FeedItem;

pub struct App {
    /// De-duplicated, reverse-chronological items.
    pub items: Vec<FeedItem>,
    /// Fast lookup to avoid inserting duplicates.
    seen: HashMap<String, ()>,
    /// List selection state for scrolling.
    pub list_state: ListState,
    /// Whether the user has requested to quit.
    pub quit: bool,
    /// Last poll status message.
    pub status: String,
}

impl App {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            seen: HashMap::new(),
            list_state: ListState::default(),
            quit: false,
            status: "Starting…".into(),
        }
    }

    /// Merge newly-fetched items, de-duplicate, and re-sort.
    pub fn merge_items(&mut self, new_items: Vec<FeedItem>) {
        for item in new_items {
            if !self.seen.contains_key(&item.id) {
                self.seen.insert(item.id.clone(), ());
                self.items.push(item);
            }
        }
        self.items.sort(); // uses Ord impl (reverse-chronological)
    }

    // -- navigation ----------------------------------------------------------

    pub fn select_next(&mut self) {
        if self.items.is_empty() {
            return;
        }
        let i = match self.list_state.selected() {
            Some(i) => (i + 1).min(self.items.len() - 1),
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    pub fn select_previous(&mut self) {
        if self.items.is_empty() {
            return;
        }
        let i = match self.list_state.selected() {
            Some(i) => i.saturating_sub(1),
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    pub fn select_first(&mut self) {
        if !self.items.is_empty() {
            self.list_state.select(Some(0));
        }
    }

    pub fn select_last(&mut self) {
        if !self.items.is_empty() {
            self.list_state.select(Some(self.items.len() - 1));
        }
    }

    // -- rendering -----------------------------------------------------------

    pub fn draw(&mut self, frame: &mut Frame) {
        let [main_area, status_area] = Layout::vertical([
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .areas(frame.area());

        self.draw_list(frame, main_area);
        self.draw_status(frame, status_area);
    }

    fn draw_list(&mut self, frame: &mut Frame, area: Rect) {
        let list_items: Vec<ListItem> = self
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
                    Span::styled(
                        &item.title,
                        Style::default().fg(Color::White),
                    ),
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

        frame.render_stateful_widget(list, area, &mut self.list_state);
    }

    fn draw_status(&self, frame: &mut Frame, area: Rect) {
        let status = Paragraph::new(Line::from(vec![
            Span::styled(" ", Style::default()),
            Span::styled(
                &self.status,
                Style::default().fg(Color::Yellow),
            ),
            Span::raw("  "),
            Span::styled(
                format!("{} items", self.items.len()),
                Style::default().fg(Color::Green),
            ),
            Span::raw("  q: quit  ↑/↓: scroll  Home/End: jump"),
        ]));
        frame.render_widget(status, area);
    }
}
