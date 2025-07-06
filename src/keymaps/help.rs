use ratatui::crossterm::event::KeyEvent;

use crate::App;
use crate::commands::{Context, HELP_BINDINGS, lookup_and_run};

pub fn handle(app: &mut App, key: KeyEvent, ctx: &mut Context) -> bool {
    lookup_and_run(HELP_BINDINGS, key, app, ctx)
}
