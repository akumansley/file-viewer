use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::{App, Mode};

pub fn handle(app: &mut App, query: &mut String, key: KeyEvent, height: u16) -> bool {
    match key.code {
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.clear_search();
            app.mode = Mode::Normal;
        }
        KeyCode::Esc => app.mode = Mode::Normal,
        KeyCode::Enter => {
            let q = query.clone();
            app.set_search_query(q);
            app.mode = Mode::Normal;
            app.ensure_visible(height);
        }
        KeyCode::Backspace => {
            query.pop();
        }
        KeyCode::Char(c) => {
            query.push(c);
        }
        _ => {}
    }
    false
}
