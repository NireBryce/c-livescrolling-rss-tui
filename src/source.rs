use anyhow::Result;
use chrono::{DateTime, Utc};
use std::cmp::Ordering;

/// A single item from any data source.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct FeedItem {
    /// Unique identifier (e.g. RSS guid or URL).
    pub id: String,
    /// Display title.
    pub title: String,
    /// Optional longer description / summary.
    pub description: Option<String>,
    /// Link to the full content.
    pub link: Option<String>,
    /// Publication timestamp (used for sorting).
    pub published: Option<DateTime<Utc>>,
    /// Name of the source / feed this came from.
    pub source_name: String,
}

impl Ord for FeedItem {
    fn cmp(&self, other: &Self) -> Ordering {
        // Reverse chronological: newer items first.
        other.published.cmp(&self.published)
    }
}

impl PartialOrd for FeedItem {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Trait that any data source must implement.
///
/// To add a new source (Atom, JSON feed, API, etc.) just implement this trait
/// and wire it into the polling loop.
pub trait DataSource: Send {
    /// Human-readable name for this source.
    fn name(&self) -> &str;

    /// Fetch the latest batch of items.  The implementation should do its own
    /// HTTP / IO work and return parsed items.
    fn fetch(&self) -> Result<Vec<FeedItem>>;
}

// ---------------------------------------------------------------------------
// RSS implementation
// ---------------------------------------------------------------------------

pub struct RssSource {
    pub url: String,
    pub label: String,
}

impl RssSource {
    pub fn new(url: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            label: label.into(),
        }
    }
}

impl RssSource {
    /// Parse an RSS channel into `FeedItem`s.  Extracted so tests can call it
    /// without hitting the network.
    pub fn parse_channel(channel: &rss::Channel, label: &str) -> Vec<FeedItem> {
        channel
            .items()
            .iter()
            .map(|item| {
                let id = item
                    .guid()
                    .map(|g| g.value().to_string())
                    .or_else(|| item.link().map(String::from))
                    .unwrap_or_default();

                let published = item
                    .pub_date()
                    .and_then(|d| DateTime::parse_from_rfc2822(d).ok())
                    .map(|dt| dt.with_timezone(&Utc));

                FeedItem {
                    id,
                    title: item.title().unwrap_or("(untitled)").to_string(),
                    description: item.description().map(String::from),
                    link: item.link().map(String::from),
                    published,
                    source_name: label.to_string(),
                }
            })
            .collect()
    }
}

impl DataSource for RssSource {
    fn name(&self) -> &str {
        &self.label
    }

    fn fetch(&self) -> Result<Vec<FeedItem>> {
        let body = reqwest::blocking::get(&self.url)?.bytes()?;
        let channel = rss::Channel::read_from(&body[..])?;
        Ok(Self::parse_channel(&channel, &self.label))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    /// Helper to build a FeedItem with minimal boilerplate.
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

    // -- FeedItem ordering ---------------------------------------------------

    #[test]
    fn feed_items_sort_reverse_chronological() {
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
    fn items_without_date_sort_after_dated_items() {
        let dated = make_item("1", "Dated", Some(Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap()));
        let undated = make_item("2", "Undated", None);

        let mut items = vec![undated.clone(), dated.clone()];
        items.sort();

        assert_eq!(items[0].id, "1", "dated item should come first");
        assert_eq!(items[1].id, "2", "undated item should come last");
    }

    #[test]
    fn items_with_same_date_are_equal_ordering() {
        let ts = Utc.with_ymd_and_hms(2025, 6, 1, 12, 0, 0).unwrap();
        let a = make_item("a", "A", Some(ts));
        let b = make_item("b", "B", Some(ts));
        assert_eq!(a.cmp(&b), Ordering::Equal);
    }

    // -- RSS XML parsing -----------------------------------------------------

    #[test]
    fn parse_channel_extracts_items() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0">
  <channel>
    <title>Test Feed</title>
    <item>
      <title>First Post</title>
      <link>https://example.com/1</link>
      <guid>guid-1</guid>
      <pubDate>Mon, 01 Jan 2024 00:00:00 +0000</pubDate>
      <description>First description</description>
    </item>
    <item>
      <title>Second Post</title>
      <link>https://example.com/2</link>
      <guid>guid-2</guid>
      <pubDate>Tue, 02 Jan 2024 12:00:00 +0000</pubDate>
    </item>
  </channel>
</rss>"#;

        let channel = rss::Channel::read_from(xml.as_bytes()).unwrap();
        let items = RssSource::parse_channel(&channel, "TestFeed");

        assert_eq!(items.len(), 2);

        assert_eq!(items[0].id, "guid-1");
        assert_eq!(items[0].title, "First Post");
        assert_eq!(items[0].link.as_deref(), Some("https://example.com/1"));
        assert_eq!(items[0].description.as_deref(), Some("First description"));
        assert_eq!(items[0].source_name, "TestFeed");
        assert!(items[0].published.is_some());

        assert_eq!(items[1].id, "guid-2");
        assert_eq!(items[1].title, "Second Post");
        assert!(items[1].description.is_none());
    }

    #[test]
    fn parse_channel_falls_back_to_link_for_id() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0">
  <channel>
    <title>Test</title>
    <item>
      <title>No GUID</title>
      <link>https://example.com/no-guid</link>
    </item>
  </channel>
</rss>"#;

        let channel = rss::Channel::read_from(xml.as_bytes()).unwrap();
        let items = RssSource::parse_channel(&channel, "t");

        assert_eq!(items[0].id, "https://example.com/no-guid");
    }

    #[test]
    fn parse_channel_handles_missing_title() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0">
  <channel>
    <title>Test</title>
    <item>
      <guid>g1</guid>
    </item>
  </channel>
</rss>"#;

        let channel = rss::Channel::read_from(xml.as_bytes()).unwrap();
        let items = RssSource::parse_channel(&channel, "t");

        assert_eq!(items[0].title, "(untitled)");
    }

    #[test]
    fn parse_channel_handles_invalid_date() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0">
  <channel>
    <title>Test</title>
    <item>
      <guid>g1</guid>
      <title>Bad Date</title>
      <pubDate>not-a-real-date</pubDate>
    </item>
  </channel>
</rss>"#;

        let channel = rss::Channel::read_from(xml.as_bytes()).unwrap();
        let items = RssSource::parse_channel(&channel, "t");

        assert!(items[0].published.is_none());
    }

    // -- DataSource trait on RssSource ---------------------------------------

    #[test]
    fn rss_source_name_returns_label() {
        let src = RssSource::new("http://example.com/feed", "My Feed");
        assert_eq!(src.name(), "My Feed");
    }
}
