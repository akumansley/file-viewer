use ratatui::crossterm::event::{KeyCode, KeyEvent};

use crate::{App, Mode};

pub fn handle(app: &mut App, key: KeyEvent, _height: u16) -> bool {
    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => {
            app.mode = Mode::Normal;
        }
        _ => {}
    }
    false
}
