use anyhow::Result;
use clap::Parser;
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    crossterm::{
        event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
        execute,
        terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
    },
    prelude::*,
    widgets::{Paragraph, Wrap},
};
use std::{
    fs::File,
    io::{self, Read},
    path::PathBuf,
};

fn is_keyword(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

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

    fn ensure_visible(&mut self, height: u16) {
        if (self.cursor_y as u16) >= self.scroll + height {
            self.scroll = self.cursor_y as u16 - height + 1;
        }
        if (self.cursor_y as u16) < self.scroll {
            self.scroll = self.cursor_y as u16;
        }
    }

    fn char_at(&self, y: usize, x: usize) -> Option<u8> {
        self.lines.get(y).and_then(|l| l.as_bytes().get(x)).copied()
    }

    fn char_before(&self, y: usize, x: usize) -> Option<u8> {
        if x > 0 {
            return self
                .lines
                .get(y)
                .and_then(|l| l.as_bytes().get(x - 1))
                .copied();
        }
        if y > 0 {
            return self.lines.get(y - 1)?.as_bytes().last().copied();
        }
        None
    }

    fn skip_forward<F>(&self, y: &mut usize, x: &mut usize, pred: F)
    where
        F: Fn(u8) -> bool,
    {
        while *y < self.lines.len() {
            let bytes = self.lines[*y].as_bytes();
            while *x < bytes.len() && pred(bytes[*x]) {
                *x += 1;
            }
            if *x < bytes.len() {
                return;
            }
            if *y + 1 == self.lines.len() {
                return;
            }
            *y += 1;
            *x = 0;
        }
    }

    fn skip_backward<F>(&self, y: &mut usize, x: &mut usize, pred: F)
    where
        F: Fn(u8) -> bool,
    {
        loop {
            if *y == 0 && *x == 0 {
                return;
            }
            if *x == 0 {
                *y -= 1;
                *x = self.lines[*y].len();
                if *x == 0 {
                    continue;
                }
            }
            let bytes = self.lines[*y].as_bytes();
            while *x > 0 && pred(bytes[*x - 1]) {
                *x -= 1;
            }
            return;
        }
    }

    fn move_word_forward(&mut self) {
        let mut y = self.cursor_y;
        let mut x = self.cursor_x;

        if let Some(c) = self.char_at(y, x) {
            if c.is_ascii_whitespace() {
                self.skip_forward(&mut y, &mut x, |b| b.is_ascii_whitespace());
            } else if is_keyword(c) {
                self.skip_forward(&mut y, &mut x, |b| is_keyword(b));
            } else {
                self.skip_forward(&mut y, &mut x, |b| {
                    !is_keyword(b) && !b.is_ascii_whitespace()
                });
            }
        }

        self.skip_forward(&mut y, &mut x, |b| b.is_ascii_whitespace());

        self.cursor_y = y.min(self.lines.len().saturating_sub(1));
        self.cursor_x = x.min(self.line_len(self.cursor_y));
    }

    fn move_word_backward(&mut self) {
        if self.cursor_y == 0 && self.cursor_x == 0 {
            return;
        }

        let mut y = self.cursor_y;
        let mut x = self.cursor_x;

        self.skip_backward(&mut y, &mut x, |b| b.is_ascii_whitespace());

        if let Some(c) = self.char_before(y, x) {
            if is_keyword(c) {
                self.skip_backward(&mut y, &mut x, |b| is_keyword(b));
            } else {
                self.skip_backward(&mut y, &mut x, |b| {
                    !is_keyword(b) && !b.is_ascii_whitespace()
                });
            }
        }

        self.cursor_y = y;
        self.cursor_x = x;
    }

    fn move_paragraph_down(&mut self) {
        for i in self.cursor_y + 1..self.lines.len() {
            if self.lines[i].trim().is_empty() {
                self.cursor_y = i;
                self.cursor_x = 0;
                return;
            }
        }
        self.cursor_y = self.lines.len() - 1;
        self.cursor_x = 0;
    }

    fn move_paragraph_up(&mut self) {
        if self.cursor_y == 0 {
            return;
        }
        for i in (0..self.cursor_y).rev() {
            if self.lines[i].trim().is_empty() {
                self.cursor_y = i;
                self.cursor_x = 0;
                return;
            }
            if i == 0 {
                break;
            }
        }
        self.cursor_y = 0;
        self.cursor_x = 0;
    }

    fn half_page_down(&mut self, height: u16) {
        for _ in 0..height / 2 {
            self.move_down(height);
        }
    }

    fn half_page_up(&mut self, height: u16) {
        for _ in 0..height / 2 {
            self.move_up();
        }
    }

    fn cursor_top(&mut self) {
        self.cursor_y = self.scroll as usize;
        let len = self.line_len(self.cursor_y);
        if self.cursor_x > len {
            self.cursor_x = len;
        }
    }

    fn cursor_middle(&mut self, height: u16) {
        let mid = (self.scroll + height / 2).min(self.lines.len() as u16 - 1);
        self.cursor_y = mid as usize;
        let len = self.line_len(self.cursor_y);
        if self.cursor_x > len {
            self.cursor_x = len;
        }
    }

    fn cursor_bottom(&mut self, height: u16) {
        let bottom = (self.scroll + height - 1).min(self.lines.len() as u16 - 1);
        self.cursor_y = bottom as usize;
        let len = self.line_len(self.cursor_y);
        if self.cursor_x > len {
            self.cursor_x = len;
        }
    }

    fn goto_first_line(&mut self) {
        self.cursor_y = 0;
        self.cursor_x = 0;
    }

    fn goto_last_line(&mut self) {
        if !self.lines.is_empty() {
            self.cursor_y = self.lines.len() - 1;
            self.cursor_x = 0;
        }
    }
}

#[derive(Parser)]
struct Cli {
    /// Print the file without launching the TUI
    #[arg(long)]
    headless: bool,

    /// Path to the file to view
    path: PathBuf,
}

fn main() -> Result<()> {
    let args = Cli::parse();
    let mut file = File::open(&args.path)?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;

    if args.headless {
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
    let mut pending_g = false;
    loop {
        terminal.draw(|f| ui(f, &app))?;

        if let Event::Key(key) = event::read()? {
            let height = terminal.size()?.height;
            if pending_g {
                if let KeyCode::Char('g') = key.code {
                    app.goto_first_line();
                    app.ensure_visible(height);
                    pending_g = false;
                    continue;
                }
                pending_g = false;
            }
            match key.code {
                KeyCode::Char('q') => return Ok(()),
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
                KeyCode::Char('g') => {
                    pending_g = true;
                }
                KeyCode::Char('G') => {
                    app.goto_last_line();
                    app.ensure_visible(height);
                }
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
    f.set_cursor_position((cursor_x, cursor_y));
}
#[cfg(test)]
mod tests {
    use super::*;
    use insta::assert_snapshot;
    use ratatui::{Terminal, backend::TestBackend};

    #[test]
    fn initial_ui_snapshot() {
        let content = "hello\nworld".to_string();
        let app = App::new(content);
        let backend = TestBackend::new(20, 5);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| ui(f, &app)).unwrap();
        assert_snapshot!(terminal.backend());
    }

    #[test]
    fn scrolling_ctrl_d_and_ctrl_u() {
        // Build content with many lines so we can scroll
        let content: String = (1..=20)
            .map(|i| format!("line {i}\n"))
            .collect();
        let mut app = App::new(content);
        let backend = TestBackend::new(20, 5);
        let mut terminal = Terminal::new(backend).unwrap();
        let height = terminal.size().unwrap().height;

        // Scroll down using Ctrl-D three times to move the viewport
        for _ in 0..3 {
            app.half_page_down(height);
        }
        terminal.draw(|f| ui(f, &app)).unwrap();
        assert_snapshot!("after_ctrl_d", terminal.backend());

        // Scroll back up using Ctrl-U three times
        for _ in 0..3 {
            app.half_page_up(height);
        }
        terminal.draw(|f| ui(f, &app)).unwrap();
        assert_snapshot!("after_ctrl_u", terminal.backend());
    }
}
