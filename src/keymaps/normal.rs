use ratatui::crossterm::event::KeyEvent;

use crate::App;
use crate::commands::{Context, NORMAL_BINDINGS, lookup_and_run};

pub fn handle(app: &mut App, key: KeyEvent, ctx: &mut Context) -> bool {
    lookup_and_run(NORMAL_BINDINGS, key, app, ctx)
}
