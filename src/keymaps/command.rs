use ratatui::crossterm::event::KeyEvent;

use crate::App;
use crate::commands::{COMMAND_BINDINGS, Context, EditorCommand, lookup_and_run};

pub fn handle(app: &mut App, key: KeyEvent, ctx: &mut Context) -> bool {
    if let Some(c) = match key.code {
        ratatui::crossterm::event::KeyCode::Char(ch) => Some(ch),
        _ => None,
    } {
        // typed characters should be appended to command string
        if COMMAND_BINDINGS
            .iter()
            .any(|b| b.key.code == key.code && b.key.modifiers == key.modifiers)
        {
            return lookup_and_run(COMMAND_BINDINGS, key, app, ctx);
        } else {
            return EditorCommand::CommandChar(c).run(app, ctx);
        }
    }
    lookup_and_run(COMMAND_BINDINGS, key, app, ctx)
}
