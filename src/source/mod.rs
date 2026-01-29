//! Data source abstraction layer.
//!
//! This module defines the [`DataSource`] trait and the common [`FeedItem`]
//! type.  Concrete source implementations live in sub-modules (currently only
//! [`rss`]).
//!
//! ## For contributors — adding a new source
//!
//! 1. Create a new file in this directory (e.g. `atom.rs`).
//! 2. Define a struct (e.g. `AtomSource`) and implement [`DataSource`] for it.
//! 3. Add `mod atom;` below and re-export your struct in the `pub use` block.
//! 4. Construct an instance in `main.rs` and add it to the `sources` vec.
//!
//! That's it — the polling loop, de-duplication, and UI are all source-agnostic.

mod feed_item;
mod rss;

// Re-export the public API of this module so callers can write
// `use crate::source::{DataSource, FeedItem, RssSource};`
pub use feed_item::FeedItem;
pub use rss::RssSource;

use anyhow::Result;

/// Trait that every data source must implement.
///
/// The polling loop calls [`fetch()`](DataSource::fetch) periodically on a
/// background thread, so implementations must be [`Send`].
///
/// ## Implementing a new source
///
/// ```ignore
/// pub struct MySource { /* config fields */ }
///
/// impl DataSource for MySource {
///     fn name(&self) -> &str { "my-source" }
///
///     fn fetch(&self) -> Result<Vec<FeedItem>> {
///         // Perform HTTP / IO, then convert into FeedItem values.
///         todo!()
///     }
/// }
/// ```
pub trait DataSource: Send {
    /// Human-readable label shown in the status bar / alongside items.
    fn name(&self) -> &str;

    /// Fetch the latest batch of items.
    ///
    /// Implementations should perform their own HTTP/IO work and return
    /// parsed [`FeedItem`] values.  Errors are propagated to the UI as
    /// status messages.
    fn fetch(&self) -> Result<Vec<FeedItem>>;
}
