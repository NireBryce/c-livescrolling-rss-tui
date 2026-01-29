mod app;
mod source;

use std::{
    io,
    sync::mpsc,
    thread,
    time::Duration,
};

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use app::App;
use source::{DataSource, FeedItem, RssSource};

/// Messages sent from the poller thread to the UI thread.
enum PollMsg {
    Items(Vec<FeedItem>),
    Error(String),
}

fn main() -> Result<()> {
    let url = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "https://feeds.bbci.co.uk/news/rss.xml".into());

    // --- set up data sources ------------------------------------------------
    let sources: Vec<Box<dyn DataSource>> = vec![
        Box::new(RssSource::new(&url, "RSS")),
    ];

    // --- background polling -------------------------------------------------
    let (tx, rx) = mpsc::channel::<PollMsg>();
    let poll_interval = Duration::from_secs(60);

    thread::spawn(move || {
        loop {
            for src in &sources {
                match src.fetch() {
                    Ok(items) => { let _ = tx.send(PollMsg::Items(items)); }
                    Err(e) => { let _ = tx.send(PollMsg::Error(format!("{}: {e}", src.name()))); }
                }
            }
            thread::sleep(poll_interval);
        }
    });

    // --- terminal setup -----------------------------------------------------
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();

    // --- main loop ----------------------------------------------------------
    let tick_rate = Duration::from_millis(100);

    loop {
        // drain all pending messages from poller
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

        terminal.draw(|f| app.draw(f))?;

        if event::poll(tick_rate)? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => {
                        app.quit = true;
                    }
                    KeyCode::Down | KeyCode::Char('j') => app.select_next(),
                    KeyCode::Up | KeyCode::Char('k') => app.select_previous(),
                    KeyCode::Home | KeyCode::Char('g') => app.select_first(),
                    KeyCode::End | KeyCode::Char('G') => app.select_last(),
                    _ => {}
                }
            }
        }

        if app.quit {
            break;
        }
    }

    // --- teardown -----------------------------------------------------------
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}
