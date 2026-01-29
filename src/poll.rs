//! Background feed polling.
//!
//! Runs on a dedicated thread, periodically fetching all configured data
//! sources and sending results to the UI thread over an [`mpsc`] channel.
//!
//! ## For contributors
//!
//! The poller is intentionally simple: it loops forever, fetches every source
//! sequentially, sends results, then sleeps.  If you need concurrent fetching
//! of multiple sources, consider spawning one thread per source or switching
//! to async with [`tokio`].

use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use crate::source::{DataSource, FeedItem};

/// Messages sent from the poller thread to the UI thread.
pub enum PollMsg {
    /// A successful fetch returned these items.
    Items(Vec<FeedItem>),
    /// A fetch failed with this error description.
    Error(String),
}

/// How often the poller re-fetches all sources.
const POLL_INTERVAL: Duration = Duration::from_secs(60);

/// Spawn the background polling thread.
///
/// Returns a receiver that the main loop should drain on every tick.
/// The thread runs until the process exits (there is no explicit shutdown
/// signal — the channel closes when the receiver is dropped).
pub fn spawn(sources: Vec<Box<dyn DataSource>>) -> mpsc::Receiver<PollMsg> {
    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        loop {
            for src in &sources {
                let msg = match src.fetch() {
                    Ok(items) => PollMsg::Items(items),
                    Err(e) => PollMsg::Error(format!("{}: {e}", src.name())),
                };
                // If the receiver is gone the main thread has exited;
                // silently stop polling.
                if tx.send(msg).is_err() {
                    return;
                }
            }
            thread::sleep(POLL_INTERVAL);
        }
    });

    rx
}
