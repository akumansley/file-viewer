use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::{App, Mode};

pub fn handle(app: &mut App, cmd: &mut String, key: KeyEvent, _height: u16) -> bool {
    match key.code {
        KeyCode::Esc => app.mode = Mode::Normal,
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.mode = Mode::Normal;
        }
        KeyCode::Enter => match cmd.trim() {
            "q" => return true,
            "help" => {
                app.mode = Mode::Help;
            }
            _ => {
                app.mode = Mode::Normal;
            }
        },
        KeyCode::Backspace => {
            cmd.pop();
        }
        KeyCode::Char(c) => {
            cmd.push(c);
        }
        _ => {}
    }
    false
}
