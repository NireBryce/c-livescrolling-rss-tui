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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    fn make_item(id: &str, title: &str, published: Option<chrono::DateTime<Utc>>) -> FeedItem {
        FeedItem {
            id: id.to_string(),
            title: title.to_string(),
            description: None,
            link: None,
            published,
            source_name: "test".to_string(),
        }
    }

    fn sample_items() -> Vec<FeedItem> {
        vec![
            make_item("1", "Old", Some(Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap())),
            make_item("2", "Mid", Some(Utc.with_ymd_and_hms(2025, 6, 1, 0, 0, 0).unwrap())),
            make_item("3", "New", Some(Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap())),
        ]
    }

    // -- construction --------------------------------------------------------

    #[test]
    fn new_app_starts_empty() {
        let app = App::new();
        assert!(app.items.is_empty());
        assert!(!app.quit);
        assert!(app.list_state.selected().is_none());
    }

    // -- merge_items ---------------------------------------------------------

    #[test]
    fn merge_items_inserts_and_sorts_reverse_chronological() {
        let mut app = App::new();
        app.merge_items(sample_items());

        assert_eq!(app.items.len(), 3);
        assert_eq!(app.items[0].id, "3", "newest first");
        assert_eq!(app.items[1].id, "2");
        assert_eq!(app.items[2].id, "1", "oldest last");
    }

    #[test]
    fn merge_items_deduplicates_by_id() {
        let mut app = App::new();
        app.merge_items(vec![
            make_item("dup", "First", Some(Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap())),
        ]);
        app.merge_items(vec![
            make_item("dup", "Second copy", Some(Utc.with_ymd_and_hms(2025, 1, 2, 0, 0, 0).unwrap())),
            make_item("new", "New item", Some(Utc.with_ymd_and_hms(2025, 1, 3, 0, 0, 0).unwrap())),
        ]);

        assert_eq!(app.items.len(), 2);
        // The original "First" title is kept, not overwritten.
        assert!(app.items.iter().any(|i| i.id == "dup" && i.title == "First"));
        assert!(app.items.iter().any(|i| i.id == "new"));
    }

    #[test]
    fn merge_items_handles_empty_input() {
        let mut app = App::new();
        app.merge_items(vec![]);
        assert!(app.items.is_empty());
    }

    #[test]
    fn merge_items_preserves_existing_on_second_call() {
        let mut app = App::new();
        app.merge_items(vec![make_item("a", "A", None)]);
        app.merge_items(vec![make_item("b", "B", None)]);
        assert_eq!(app.items.len(), 2);
    }

    // -- navigation ----------------------------------------------------------

    #[test]
    fn select_next_on_empty_is_noop() {
        let mut app = App::new();
        app.select_next();
        assert!(app.list_state.selected().is_none());
    }

    #[test]
    fn select_previous_on_empty_is_noop() {
        let mut app = App::new();
        app.select_previous();
        assert!(app.list_state.selected().is_none());
    }

    #[test]
    fn select_first_on_empty_is_noop() {
        let mut app = App::new();
        app.select_first();
        assert!(app.list_state.selected().is_none());
    }

    #[test]
    fn select_last_on_empty_is_noop() {
        let mut app = App::new();
        app.select_last();
        assert!(app.list_state.selected().is_none());
    }

    #[test]
    fn select_next_starts_at_zero_then_advances() {
        let mut app = App::new();
        app.merge_items(sample_items());

        app.select_next();
        assert_eq!(app.list_state.selected(), Some(0));

        app.select_next();
        assert_eq!(app.list_state.selected(), Some(1));

        app.select_next();
        assert_eq!(app.list_state.selected(), Some(2));
    }

    #[test]
    fn select_next_clamps_at_last_item() {
        let mut app = App::new();
        app.merge_items(sample_items());

        app.select_last();
        app.select_next();
        assert_eq!(app.list_state.selected(), Some(2));
    }

    #[test]
    fn select_previous_clamps_at_zero() {
        let mut app = App::new();
        app.merge_items(sample_items());

        app.select_first();
        app.select_previous();
        assert_eq!(app.list_state.selected(), Some(0));
    }

    #[test]
    fn select_previous_moves_up() {
        let mut app = App::new();
        app.merge_items(sample_items());

        app.select_last(); // index 2
        app.select_previous();
        assert_eq!(app.list_state.selected(), Some(1));
    }

    #[test]
    fn select_first_jumps_to_zero() {
        let mut app = App::new();
        app.merge_items(sample_items());

        app.select_last();
        app.select_first();
        assert_eq!(app.list_state.selected(), Some(0));
    }

    #[test]
    fn select_last_jumps_to_end() {
        let mut app = App::new();
        app.merge_items(sample_items());

        app.select_last();
        assert_eq!(app.list_state.selected(), Some(2));
    }

    // -- rendering (smoke tests) ---------------------------------------------

    #[test]
    fn draw_does_not_panic_with_no_items() {
        let mut app = App::new();
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| app.draw(f)).unwrap();
    }

    #[test]
    fn draw_does_not_panic_with_items() {
        let mut app = App::new();
        app.merge_items(sample_items());
        app.select_first();

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| app.draw(f)).unwrap();
    }

    #[test]
    fn draw_status_shows_item_count() {
        let mut app = App::new();
        app.merge_items(sample_items());
        app.status = "OK".to_string();

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| app.draw(f)).unwrap();

        let buf = terminal.backend().buffer().clone();
        let text: String = buf.content().iter().map(|c| c.symbol().chars().next().unwrap_or(' ')).collect();
        assert!(text.contains("3 items"), "status bar should show item count");
    }
}
