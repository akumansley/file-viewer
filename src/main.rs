use anyhow::Result;
use ratatui::{
    backend::CrosstermBackend,
    crossterm::{
        event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
        execute,
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    },
    prelude::*,
    widgets::{Paragraph, Wrap},
    Terminal,
};
use std::{
    fs::File,
    io::{self, Read},
    path::PathBuf,
};

struct App {
    lines: Vec<String>,
    cursor_x: usize,
    cursor_y: usize,
    scroll: u16,
}

impl App {
    fn new(content: String) -> Self {
        let lines = content.lines().map(|s| s.to_string()).collect();
        Self {
            lines,
            cursor_x: 0,
            cursor_y: 0,
            scroll: 0,
        }
    }

    fn content(&self) -> String {
        self.lines.join("\n")
    }

    fn line_len(&self, line: usize) -> usize {
        self.lines.get(line).map(|l| l.len()).unwrap_or(0)
    }

    fn move_left(&mut self) {
        if self.cursor_x > 0 {
            self.cursor_x -= 1;
        }
    }

    fn move_right(&mut self) {
        let len = self.line_len(self.cursor_y);
        if self.cursor_x < len {
            self.cursor_x += 1;
        }
    }

    fn move_down(&mut self, height: u16) {
        if self.cursor_y + 1 < self.lines.len() {
            self.cursor_y += 1;
            if (self.cursor_y as u16) >= self.scroll + height {
                self.scroll = self.cursor_y as u16 - height + 1;
            }
            let len = self.line_len(self.cursor_y);
            if self.cursor_x > len {
                self.cursor_x = len;
            }
        }
    }

    fn move_up(&mut self) {
        if self.cursor_y > 0 {
            self.cursor_y -= 1;
            if (self.cursor_y as u16) < self.scroll {
                self.scroll = self.cursor_y as u16;
            }
            let len = self.line_len(self.cursor_y);
            if self.cursor_x > len {
                self.cursor_x = len;
            }
        }
    }
}

fn main() -> Result<()> {
    let headless = std::env::args().any(|arg| arg == "--headless");
    let path = std::env::args().nth(1).expect("no file given");
    let mut file = File::open(PathBuf::from(path))?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;

    if headless {
        println!("{}", content);
        return Ok(());
    }

    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let res = run_app(&mut terminal, content);

    // restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err)
    }

    Ok(())
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, content: String) -> io::Result<()> {
    let mut app = App::new(content);
    loop {
        terminal.draw(|f| ui(f, &app))?;

        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Char('q') => return Ok(()),
                KeyCode::Char('h') => app.move_left(),
                KeyCode::Char('j') => {
                    let height = terminal.size()?.height;
                    app.move_down(height);
                }
                KeyCode::Char('k') => app.move_up(),
                KeyCode::Char('l') => app.move_right(),
                _ => {}
            }
        }
    }
}

fn ui(f: &mut Frame, app: &App) {
    let area = f.area();
    let paragraph = Paragraph::new(app.content())
        .wrap(Wrap { trim: true })
        .scroll((app.scroll, 0));
    f.render_widget(paragraph, area);
    let cursor_y = area.y + (app.cursor_y as u16).saturating_sub(app.scroll);
    let cursor_x = area.x + app.cursor_x as u16;
    f.set_cursor(cursor_x, cursor_y);
}
#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{backend::TestBackend, Terminal};
    use insta::assert_display_snapshot;

    #[test]
    fn initial_ui_snapshot() {
        let content = "hello\nworld".to_string();
        let app = App::new(content);
        let backend = TestBackend::new(20, 5);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| ui(f, &app)).unwrap();
        assert_display_snapshot!(terminal.backend());
    }
}

