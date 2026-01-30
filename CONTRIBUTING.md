# Contributing to livescroll-rss

Thank you for your interest in contributing!  This document will help you
get started quickly.

## Getting started

### Prerequisites

- [Rust](https://rustup.rs/) 1.70+
- A terminal emulator that supports ANSI colours

### Building and running

```sh
cargo build            # debug build
cargo run              # run with default feed
cargo run -- URL       # run with a custom feed
cargo test             # run the full test suite
```

### Running a single test

```sh
cargo test merge_deduplicates    # runs any test matching that substring
```

## Project architecture

```
src/
├── main.rs            Wires everything together (args, terminal, event loop)
├── app.rs             Application state — the single source of truth
├── ui.rs              Rendering logic (reads App, draws ratatui widgets)
├── input.rs           Key event → App action mapping
├── poll.rs            Background thread that fetches sources on a timer
└── source/
    ├── mod.rs         DataSource trait + re-exports
    ├── feed_item.rs   FeedItem struct shared by all sources
    └── rss.rs         RSS 2.0 implementation (use as a template)
```

Data flows in one direction each tick:

```
poll → (channel) → main loop → app.merge_items()
                             → ui::draw()
keyboard → input::handle_key_event() → app mutations
```

Each module has a single responsibility:

| Module       | Owns                           | Does NOT do           |
|--------------|--------------------------------|-----------------------|
| `app.rs`     | State, de-duplication, sorting | I/O, rendering        |
| `ui.rs`      | Widget layout, colours         | State mutation, I/O   |
| `input.rs`   | Key → action mapping           | Rendering, I/O        |
| `poll.rs`    | Background fetching, channel   | State, rendering      |
| `source/*`   | Network I/O, parsing           | State, rendering      |

## Common tasks

### Adding a new keybinding

1. If the action doesn't exist yet, add a method on `App` in `src/app.rs`.
2. Add a `KeyCode` match arm in `src/input.rs` → `handle_key_event()`.
3. Update the help text in `src/ui.rs` → `draw_status_bar()`.
4. Update the keybindings table in `README.md` and `doc/livescroll-rss.1`.
5. Add a test in `src/app.rs` for the new `App` method.

### Adding a new data source (e.g. Atom, JSON Feed)

1. Create `src/source/atom.rs` (or whatever fits).
2. Define a struct with any config your source needs:
   ```rust
   pub struct AtomSource {
       pub url: String,
       pub label: String,
   }
   ```
3. Implement `DataSource`:
   ```rust
   impl DataSource for AtomSource {
       fn name(&self) -> &str { &self.label }

       fn fetch(&self) -> Result<Vec<FeedItem>> {
           // fetch, parse, convert to FeedItem
           todo!()
       }
   }
   ```
4. In `src/source/mod.rs`:
   - Add `mod atom;`
   - Add `pub use atom::AtomSource;`
5. In `src/main.rs`, add your source to the `sources` vec.
6. Write tests — look at `src/source/rss.rs` for the pattern.

### Changing the UI layout

All rendering lives in `src/ui.rs`.  The layout is a vertical split:
- Top: scrollable `List` widget with feed items
- Bottom: one-line `Paragraph` status bar

To add a new section (e.g. a detail pane), modify `ui::draw()` to create
a three-way layout split and add a new helper function.

## Testing

The project has unit tests in the core modules (`app`, `source/feed_item`,
`source/rss`).  Tests use:

- `make_item()` helpers for building `FeedItem` values without boilerplate
- `ratatui::backend::TestBackend` for rendering smoke tests
- Raw RSS XML strings for parsing tests (no network required)

Run the full suite with:

```sh
cargo test
```

All tests must pass before a pull request will be merged.

## Code style

- Run `cargo fmt` before committing.
- Run `cargo clippy` and fix any warnings.
- Keep modules focused — if a file grows past ~200 lines of non-test code,
  consider splitting it.
- Write `//!` module-level doc comments explaining the module's purpose and
  how contributors should work with it.
- Write `///` doc comments on all public items.
- Add tests for new public methods and parsing logic.

## Pull request checklist

- [ ] `cargo test` passes
- [ ] `cargo clippy` has no warnings
- [ ] `cargo fmt --check` passes
- [ ] New public items have doc comments
- [ ] README and man page updated if user-facing behaviour changed
- [ ] Tests cover new functionality
