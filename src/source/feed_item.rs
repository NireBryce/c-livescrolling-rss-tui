//! The core data type shared across all feed sources.
//!
//! `FeedItem` represents a single entry from any data source (RSS, Atom, API,
//! etc.).  Every source implementation converts its native format into
//! `FeedItem`s so the rest of the application can stay source-agnostic.
//!
//! ## For contributors
//!
//! If you are adding a new data source you do **not** need to modify this file
//! unless your source requires extra fields.  Just construct `FeedItem` values
//! in your source's `fetch()` implementation.

use chrono::{DateTime, Utc};
use std::cmp::Ordering;

/// A single feed entry, normalised from any data source.
///
/// All sources convert their native items into this struct so that the
/// application logic (de-duplication, sorting, rendering) doesn't need to
/// know which source type produced the item.
///
/// ## Sorting
///
/// `FeedItem` implements [`Ord`] for **reverse-chronological** ordering:
/// newer items sort before older ones, and items without a date sort last.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct FeedItem {
    /// Unique identifier used for de-duplication.
    ///
    /// For RSS this is the `<guid>` element (falling back to `<link>`).
    /// Other sources should use whatever stable, unique key they provide.
    pub id: String,

    /// Human-readable headline.
    pub title: String,

    /// Optional longer description or summary text.
    pub description: Option<String>,

    /// URL to the full content.
    pub link: Option<String>,

    /// Publication timestamp, used for sorting.
    ///
    /// `None` means the source did not provide a date; such items sort after
    /// all dated items.
    pub published: Option<DateTime<Utc>>,

    /// Name of the source or feed this came from (e.g. "BBC News").
    pub source_name: String,
}

// ---------------------------------------------------------------------------
// Ordering — reverse chronological (newest first)
// ---------------------------------------------------------------------------

impl Ord for FeedItem {
    fn cmp(&self, other: &Self) -> Ordering {
        // `other` first so that `Some(newer) > Some(older)` gives us newest-first.
        // `None` is less than `Some(_)` in the standard library, so undated
        // items naturally sink to the bottom.
        other.published.cmp(&self.published)
    }
}

impl PartialOrd for FeedItem {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    /// Shorthand constructor for tests.
    pub fn make_item(id: &str, title: &str, published: Option<DateTime<Utc>>) -> FeedItem {
        FeedItem {
            id: id.to_string(),
            title: title.to_string(),
            description: None,
            link: None,
            published,
            source_name: "test".to_string(),
        }
    }

    #[test]
    fn sort_reverse_chronological() {
        let old = make_item("1", "Old", Some(Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap()));
        let mid = make_item("2", "Mid", Some(Utc.with_ymd_and_hms(2025, 6, 15, 12, 0, 0).unwrap()));
        let new = make_item("3", "New", Some(Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap()));

        let mut items = vec![old.clone(), new.clone(), mid.clone()];
        items.sort();

        assert_eq!(items[0].id, "3", "newest first");
        assert_eq!(items[1].id, "2");
        assert_eq!(items[2].id, "1", "oldest last");
    }

    #[test]
    fn undated_items_sort_after_dated() {
        let dated = make_item("1", "Dated", Some(Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap()));
        let undated = make_item("2", "Undated", None);

        let mut items = vec![undated.clone(), dated.clone()];
        items.sort();

        assert_eq!(items[0].id, "1", "dated item should come first");
        assert_eq!(items[1].id, "2", "undated item should come last");
    }

    #[test]
    fn same_date_yields_equal_ordering() {
        let ts = Utc.with_ymd_and_hms(2025, 6, 1, 12, 0, 0).unwrap();
        let a = make_item("a", "A", Some(ts));
        let b = make_item("b", "B", Some(ts));
        assert_eq!(a.cmp(&b), Ordering::Equal);
    }
}
