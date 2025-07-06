use ratatui::crossterm::event::KeyEvent;

use crate::App;
use crate::commands::{Context, VISUAL_BINDINGS, lookup_and_run};

pub fn handle(app: &mut App, key: KeyEvent, ctx: &mut Context) -> bool {
    lookup_and_run(VISUAL_BINDINGS, key, app, ctx)
}
