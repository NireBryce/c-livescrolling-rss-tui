//! livescroll-rss — a live-updating RSS feed reader for the terminal.
//!
//! ## Architecture overview
//!
//! ```text
//! ┌──────────┐  PollMsg   ┌──────────┐  draw()  ┌──────────┐
//! │  poll.rs │ ─────────► │  app.rs  │ ───────► │  ui.rs   │
//! │ (thread) │  (channel) │ (state)  │          │ (render) │
//! └──────────┘            └──────────┘          └──────────┘
//!                              ▲
//!                              │ handle_key_event()
//!                         ┌──────────┐
//!                         │ input.rs │
//!                         └──────────┘
//! ```
//!
//! * **`source/`** — the `DataSource` trait and concrete implementations
//!   (currently RSS only).
//! * **`poll`** — spawns a background thread that fetches sources on a timer.
//! * **`app`** — owns all application state (items, scroll position, etc.).
//! * **`ui`** — pure rendering: reads `App` state and draws widgets.
//! * **`input`** — maps key events to `App` mutations.
//! * **`main`** — wires everything together: parse args, set up the terminal,
//!   and run the event loop.

mod app;
mod input;
mod poll;
mod source;
mod ui;

use std::io;
use std::time::Duration;

use anyhow::Result;
use crossterm::{
    event::{self, Event},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use app::App;
use poll::PollMsg;
use source::{DataSource, RssSource};

// ---------------------------------------------------------------------------
// RAII terminal guard — idiomatic cleanup even on panic
// ---------------------------------------------------------------------------

/// Manages terminal raw-mode and alternate-screen lifetime via [`Drop`].
///
/// Constructing this struct enters raw mode + alternate screen.  When the
/// value is dropped (normally or during stack unwinding) it restores the
/// terminal.  This prevents the common TUI bug where a panic leaves the
/// terminal in a broken state.
struct TerminalGuard {
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
}

impl TerminalGuard {
    fn new() -> Result<Self> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;
        Ok(Self { terminal })
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(self.terminal.backend_mut(), LeaveAlternateScreen);
        let _ = self.terminal.show_cursor();
    }
}

/// Install a panic hook that restores the terminal before printing the
/// panic message.  Without this, a panic inside the event loop would leave
/// raw mode enabled and the alternate screen active.
fn install_panic_hook() {
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
        original_hook(info);
    }));
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

fn main() -> Result<()> {
    install_panic_hook();

    // -- parse arguments -----------------------------------------------------
    let url = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "https://feeds.bbci.co.uk/news/rss.xml".into());

    // -- configure data sources ----------------------------------------------
    // To add more feeds, push additional sources here.
    let sources: Vec<Box<dyn DataSource>> = vec![
        Box::new(RssSource::new(&url, "RSS")),
    ];

    // -- start background polling --------------------------------------------
    let rx = poll::spawn(sources);

    // -- terminal setup (RAII — Drop restores on exit or panic) --------------
    let mut guard = TerminalGuard::new()?;
    let mut app = App::new();

    // -- main event loop -----------------------------------------------------
    // Runs at ~10 fps (100 ms tick).  Each iteration:
    //   1. Drain any messages from the poller.
    //   2. Render the UI.
    //   3. Poll for keyboard input (non-blocking, up to tick_rate).
    let tick_rate = Duration::from_millis(100);

    loop {
        // 1. Process poll messages
        while let Ok(msg) = rx.try_recv() {
            match msg {
                PollMsg::Items(items) => {
                    let count = items.len();
                    app.merge_items(items);
                    app.status = format!("Fetched {count} items");
                }
                PollMsg::Error(e) => {
                    app.status = format!("Error: {e}");
                }
            }
        }

        // 2. Render
        guard.terminal.draw(|f| ui::draw(&mut app, f))?;

        // 3. Handle input
        if event::poll(tick_rate)? {
            if let Event::Key(key) = event::read()? {
                input::handle_key_event(&mut app, key);
            }
        }

        if app.quit {
            break;
        }
    }

    // `guard` is dropped here, restoring the terminal.
    Ok(())
}
