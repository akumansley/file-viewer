use ratatui::crossterm::event::KeyEvent;

use crate::App;
use crate::commands::{Context, EditorCommand, SEARCH_BINDINGS, lookup_and_run};

pub fn handle(app: &mut App, key: KeyEvent, ctx: &mut Context) -> bool {
    if let ratatui::crossterm::event::KeyCode::Char(c) = key.code {
        if SEARCH_BINDINGS.iter().any(|b| b.key == key) {
            return lookup_and_run(SEARCH_BINDINGS, key, app, ctx);
        } else {
            return EditorCommand::SearchChar(c).run(app, ctx);
        }
    }
    lookup_and_run(SEARCH_BINDINGS, key, app, ctx)
}
