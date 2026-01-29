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

impl DataSource for RssSource {
    fn name(&self) -> &str {
        &self.label
    }

    fn fetch(&self) -> Result<Vec<FeedItem>> {
        let body = reqwest::blocking::get(&self.url)?.bytes()?;
        let channel = rss::Channel::read_from(&body[..])?;

        let items = channel
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
                    source_name: self.label.clone(),
                }
            })
            .collect();

        Ok(items)
    }
}
