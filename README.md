# livescroll-rss

A terminal UI (TUI) application that polls RSS feeds and displays a
live-updating, reverse-chronological list of items.

## Installation

### From source

Requires [Rust](https://rustup.rs/) 1.70 or later.

```sh
git clone https://github.com/NireBryce/c-livescrolling-rss-tui.git
cd c-livescrolling-rss-tui
cargo build --release
```

The binary is written to `target/release/livescroll-rss`.

## Usage

```
livescroll-rss [FEED_URL]
```

| Argument   | Default                                        | Description            |
|------------|------------------------------------------------|------------------------|
| `FEED_URL` | `https://feeds.bbci.co.uk/news/rss.xml` (BBC)  | URL of an RSS 2.0 feed |

### Examples

```sh
# Watch BBC News (default)
cargo run

# Watch a custom feed
cargo run -- https://hnrss.org/frontpage

# Run the installed binary directly
livescroll-rss https://feeds.bbci.co.uk/news/technology/rss.xml
```

## Keybindings

| Key             | Action          |
|-----------------|-----------------|
| `q` / `Esc`     | Quit            |
| `↑` / `k`       | Scroll up       |
| `↓` / `j`       | Scroll down     |
| `Home` / `g`    | Jump to top     |
| `End` / `G`     | Jump to bottom  |

## How it works

1. A background thread fetches the RSS feed every 60 seconds.
2. New items are de-duplicated (by GUID or link) and merged into an
   in-memory list sorted newest-first.
3. The terminal UI redraws at ~10 fps, showing the list and a status bar.

The feed source is behind a pluggable `DataSource` trait, so new source
types (Atom, JSON Feed, REST APIs) can be added without changing the UI
or polling logic.  See [CONTRIBUTING.md](CONTRIBUTING.md) for details.

## Project layout

```
src/
├── main.rs          Entry point and event loop
├── app.rs           Application state (items, scroll, status)
├── ui.rs            Terminal rendering (ratatui widgets)
├── input.rs         Keyboard event → App action mapping
├── poll.rs          Background feed polling thread
└── source/
    ├── mod.rs       DataSource trait definition
    ├── feed_item.rs FeedItem struct (shared across all sources)
    └── rss.rs       RSS 2.0 source implementation
```

## Man page

A man page is provided at `doc/livescroll-rss.1`.  View it with:

```sh
man doc/livescroll-rss.1
```

## License

See [LICENSE](LICENSE) for details.
