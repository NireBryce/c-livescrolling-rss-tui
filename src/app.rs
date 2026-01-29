//! Application state.
//!
//! [`App`] owns the feed item list, de-duplication set, scroll position, and
//! status message.  It is the single source of truth that the UI reads from
//! and that input / polling code writes to.
//!
//! ## For contributors
//!
//! * **State only** — this module has no I/O, no rendering, and no
//!   input handling.  Those live in [`crate::ui`], [`crate::input`], and
//!   [`crate::poll`] respectively.
//! * All public methods are covered by the test suite at the bottom of
//!   this file.  Please add tests for any new behaviour.

use std::collections::HashSet;

use ratatui::widgets::ListState;

use crate::source::FeedItem;

/// Core application state.
///
/// Created once in `main()` and passed by mutable reference to the input
/// handler, poll-message processor, and UI renderer each tick.
pub struct App {
    /// De-duplicated feed items in reverse-chronological order (newest first).
    pub items: Vec<FeedItem>,

    /// Set of item IDs we have already seen, used for O(1) de-duplication.
    seen: HashSet<String>,

    /// Ratatui list widget selection state (tracks the highlighted row).
    pub list_state: ListState,

    /// Set to `true` when the user requests quit; checked by the main loop.
    pub quit: bool,

    /// Human-readable status message shown in the bottom bar
    /// (e.g. "Fetched 42 items" or "Error: timeout").
    pub status: String,
}

impl Default for App {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            seen: HashSet::new(),
            list_state: ListState::default(),
            quit: false,
            status: "Starting\u{2026}".into(), // "Starting…"
        }
    }
}

impl App {
    /// Create a new, empty application state.
    pub fn new() -> Self {
        Self::default()
    }

    // -- feed management -----------------------------------------------------

    /// Merge newly-fetched items into the list.
    ///
    /// * Duplicates (by `id`) are silently skipped.
    /// * The list is re-sorted after insertion so that the newest item is
    ///   always at index 0.
    ///
    /// Accepts any iterator of `FeedItem`s — callers can pass a `Vec`, a
    /// slice, a `drain(..)`, etc.
    pub fn merge_items(&mut self, new_items: impl IntoIterator<Item = FeedItem>) {
        for item in new_items {
            if self.seen.insert(item.id.clone()) {
                self.items.push(item);
            }
        }
        // `sort_unstable` is preferred when equal elements have no meaningful
        // relative order — it avoids an allocation and is faster.
        self.items.sort_unstable();
    }

    // -- list navigation -----------------------------------------------------

    /// Move the selection cursor down by one row.
    pub fn select_next(&mut self) {
        if self.items.is_empty() {
            return;
        }
        let i = self
            .list_state
            .selected()
            .map_or(0, |i| (i + 1).min(self.items.len() - 1));
        self.list_state.select(Some(i));
    }

    /// Move the selection cursor up by one row.
    pub fn select_previous(&mut self) {
        if self.items.is_empty() {
            return;
        }
        let i = self
            .list_state
            .selected()
            .map_or(0, |i| i.saturating_sub(1));
        self.list_state.select(Some(i));
    }

    /// Jump the selection cursor to the first item.
    pub fn select_first(&mut self) {
        if !self.items.is_empty() {
            self.list_state.select(Some(0));
        }
    }

    /// Jump the selection cursor to the last item.
    pub fn select_last(&mut self) {
        if !self.items.is_empty() {
            self.list_state.select(Some(self.items.len() - 1));
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

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
    fn merge_inserts_and_sorts_reverse_chronological() {
        let mut app = App::new();
        app.merge_items(sample_items());

        assert_eq!(app.items.len(), 3);
        assert_eq!(app.items[0].id, "3", "newest first");
        assert_eq!(app.items[1].id, "2");
        assert_eq!(app.items[2].id, "1", "oldest last");
    }

    #[test]
    fn merge_deduplicates_by_id() {
        let mut app = App::new();
        app.merge_items(vec![
            make_item("dup", "First", Some(Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap())),
        ]);
        app.merge_items(vec![
            make_item("dup", "Second copy", Some(Utc.with_ymd_and_hms(2025, 1, 2, 0, 0, 0).unwrap())),
            make_item("new", "New item", Some(Utc.with_ymd_and_hms(2025, 1, 3, 0, 0, 0).unwrap())),
        ]);

        assert_eq!(app.items.len(), 2);
        assert!(app.items.iter().any(|i| i.id == "dup" && i.title == "First"));
        assert!(app.items.iter().any(|i| i.id == "new"));
    }

    #[test]
    fn merge_handles_empty_input() {
        let mut app = App::new();
        app.merge_items(vec![]);
        assert!(app.items.is_empty());
    }

    #[test]
    fn merge_preserves_existing_on_second_call() {
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
        app.select_last();
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
        terminal.draw(|f| crate::ui::draw(&mut app, f)).unwrap();
    }

    #[test]
    fn draw_does_not_panic_with_items() {
        let mut app = App::new();
        app.merge_items(sample_items());
        app.select_first();

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| crate::ui::draw(&mut app, f)).unwrap();
    }

    #[test]
    fn draw_status_shows_item_count() {
        let mut app = App::new();
        app.merge_items(sample_items());
        app.status = "OK".to_string();

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| crate::ui::draw(&mut app, f)).unwrap();

        let buf = terminal.backend().buffer().clone();
        let text: String = buf.content().iter().map(|c| c.symbol().chars().next().unwrap_or(' ')).collect();
        assert!(text.contains("3 items"), "status bar should show item count");
    }
}
