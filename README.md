# livescroll-rss

A terminal UI that polls RSS feeds and displays a live-updating, reverse-chronological list of items.

## Usage

```sh
# Default feed (BBC News)
cargo run

# Custom feed URL
cargo run -- https://example.com/feed.xml
```

## Keybindings

| Key | Action |
|-----|--------|
| `q` / `Esc` | Quit |
| `↑` / `k` | Scroll up |
| `↓` / `j` | Scroll down |
| `Home` / `g` | Jump to top |
| `End` / `G` | Jump to bottom |

## Architecture

The data source is behind a `DataSource` trait (`src/source.rs`), making it
straightforward to add new source types (Atom, JSON Feed, APIs, etc.) without
changing the TUI or polling logic. Currently only RSS is implemented via
`RssSource`.

Polling runs on a background thread that sends items to the UI thread over a
channel. The TUI merges and de-duplicates incoming items, keeping everything
sorted in reverse chronological order.
