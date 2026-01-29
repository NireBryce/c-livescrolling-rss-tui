//! RSS feed source implementation.
//!
//! This module shows how to implement the [`DataSource`] trait for a concrete
//! feed format.  Use it as a template when adding support for Atom, JSON Feed,
//! or any other format.
//!
//! ## For contributors — adding a new source type
//!
//! 1. Create a new file under `src/source/` (e.g. `atom.rs`).
//! 2. Define a struct that holds any configuration your source needs (URL,
//!    API key, etc.).
//! 3. Implement [`DataSource`] for your struct — `name()` returns a label and
//!    `fetch()` returns `Vec<FeedItem>`.
//! 4. Re-export your struct from `src/source/mod.rs`.
//! 5. Wire it into the source list in `main.rs`.
//!
//! The RSS implementation below is a complete worked example.

use anyhow::Result;
use chrono::{DateTime, Utc};

use super::{DataSource, FeedItem};

/// An RSS feed data source.
///
/// Fetches and parses an RSS 2.0 feed over HTTP using the [`rss`] crate.
pub struct RssSource {
    /// The feed URL to poll.
    pub url: String,
    /// A human-readable label shown in the UI next to each item.
    pub label: String,
}

impl RssSource {
    /// Create a new RSS source.
    ///
    /// # Arguments
    ///
    /// * `url` — full URL of the RSS feed (e.g.
    ///   `https://feeds.bbci.co.uk/news/rss.xml`).
    /// * `label` — short name displayed in the TUI for items from this feed.
    pub fn new(url: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            label: label.into(),
        }
    }

    /// Parse an already-fetched [`rss::Channel`] into [`FeedItem`]s.
    ///
    /// This is a pure function (no I/O) so that tests can exercise the
    /// parsing logic without hitting the network.
    pub fn parse_channel(channel: &rss::Channel, label: &str) -> Vec<FeedItem> {
        channel
            .items()
            .iter()
            .map(|item| {
                // Prefer <guid>, fall back to <link>, then empty string.
                let id = item
                    .guid()
                    .map(|g| g.value().to_string())
                    .or_else(|| item.link().map(String::from))
                    .unwrap_or_default();

                // Parse RFC-2822 date; gracefully degrade to None on failure.
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
        let channel = rss::Channel::read_from(body.as_ref())?;
        Ok(Self::parse_channel(&channel, &self.label))
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

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
    fn falls_back_to_link_when_no_guid() {
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
    fn handles_missing_title() {
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
    fn handles_invalid_date() {
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

    #[test]
    fn name_returns_label() {
        let src = RssSource::new("http://example.com/feed", "My Feed");
        assert_eq!(src.name(), "My Feed");
    }
}
