//! Keyboard input handling.
//!
//! Maps terminal key events to [`App`] actions.  Adding a new keybinding is
//! a single match arm in [`handle_key_event`].
//!
//! ## For contributors
//!
//! To add a new keybinding:
//!
//! 1. Add a method on [`App`] for the action (if one doesn't exist).
//! 2. Add a `KeyCode` match arm in [`handle_key_event`] that calls it.
//! 3. Update the help text in [`crate::ui::draw_status_bar`].
//! 4. Update the keybindings table in `README.md` and the man page.

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind};

use crate::app::App;

/// Process a single key event, updating app state accordingly.
///
/// Only reacts to key-press events (ignoring release / repeat) so that each
/// physical keypress triggers exactly one action.
pub fn handle_key_event(app: &mut App, key: KeyEvent) {
    if key.kind != KeyEventKind::Press {
        return;
    }

    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => app.quit = true,
        KeyCode::Down | KeyCode::Char('j') => app.select_next(),
        KeyCode::Up | KeyCode::Char('k') => app.select_previous(),
        KeyCode::Home | KeyCode::Char('g') => app.select_first(),
        KeyCode::End | KeyCode::Char('G') => app.select_last(),
        _ => {}
    }
}
