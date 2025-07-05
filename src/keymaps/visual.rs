use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::{App, Mode};

pub fn handle(app: &mut App, key: KeyEvent, height: u16) -> bool {
    match key.code {
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.mode = Mode::Normal;
            app.selection_start = None;
        }
        KeyCode::Char('q') => return true,
        KeyCode::Char('h') => app.move_left(),
        KeyCode::Char('j') => app.move_down(height),
        KeyCode::Char('k') => app.move_up(),
        KeyCode::Char('l') => app.move_right(),
        KeyCode::Char('w') => {
            app.move_word_forward();
            app.ensure_visible(height);
        }
        KeyCode::Char('b') => {
            app.move_word_backward();
            app.ensure_visible(height);
        }
        KeyCode::Char('{') => {
            app.move_paragraph_up();
            app.ensure_visible(height);
        }
        KeyCode::Char('}') => {
            app.move_paragraph_down();
            app.ensure_visible(height);
        }
        KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.half_page_up(height);
        }
        KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.half_page_down(height);
        }
        KeyCode::Char('H') => {
            app.cursor_top();
            app.ensure_visible(height);
        }
        KeyCode::Char('M') => {
            app.cursor_middle(height);
            app.ensure_visible(height);
        }
        KeyCode::Char('L') => {
            app.cursor_bottom(height);
            app.ensure_visible(height);
        }
        _ => {}
    }
    false
}
