use anyhow::Result;
use clap::Parser;
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    crossterm::{
        event::{self, DisableMouseCapture, EnableMouseCapture, Event},
        execute,
        terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
    },
    prelude::*,
    widgets::{Paragraph, Wrap},
};
mod keymaps;
use std::{
    fs::File,
    io::{self, Read},
    path::PathBuf,
};

fn is_keyword(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

fn highlight_line<'a>(
    line: &'a str,
    line_idx: usize,
    query: Option<&str>,
    selection: Option<((usize, usize), (usize, usize))>,
    line_mode: bool,
) -> Line<'a> {
    let bytes = line.as_bytes();
    let mut styles = vec![Style::default(); bytes.len()];

    if let Some(q) = query {
        if !q.is_empty() {
            let mut start = 0;
            while let Some(pos) = line[start..].find(q) {
                for i in start + pos..start + pos + q.len() {
                    if i < styles.len() {
                        styles[i] = styles[i].bg(Color::Yellow);
                    }
                }

                // search results are highlighted via styles; spans are built later

                start += pos + q.len();
            }
        }
    }

    if let Some((start, end)) = selection {
        let ((sy, sx), (ey, ex)) = if start <= end {
            (start, end)
        } else {
            (end, start)
        };
        if line_idx >= sy && line_idx <= ey {
            if line_mode {
                for style in &mut styles {
                    *style = style.add_modifier(Modifier::REVERSED);
                }
            } else {
                let sel_start = if line_idx == sy {
                    sx.min(bytes.len())
                } else {
                    0
                };
                let sel_end = if line_idx == ey {
                    ex.min(bytes.len())
                } else {
                    bytes.len()
                };
                for i in sel_start..sel_end {
                    if i < styles.len() {
                        styles[i] = styles[i].add_modifier(Modifier::REVERSED);
                    }
                }
            }
        }
    }

    if styles.is_empty() {
        return Line::from(line.to_owned());
    }

    let mut spans = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        let mut j = i + 1;
        while j < bytes.len() && styles[j] == styles[i] {
            j += 1;
        }
        let text = &line[i..j];
        spans.push(Span::styled(text.to_string(), styles[i]));
        i = j;
    }

    Line::from(spans)
}

const HELP_TEXT: &str = "File Viewer Help\n\n?     Show this help\n:help Open help screen\nq     Quit\nEsc   Close help";

#[derive(Clone)]
enum Mode {
    Normal,
    Visual,
    VisualLine,
    Command(String),
    Search(String),
    Help,
}

struct Document {
    lines: Vec<String>,
}

impl Document {
    fn new(content: String) -> Self {
        let lines = content.lines().map(|s| s.to_string()).collect();
        Self { lines }
    }
}

struct OverlayItem {
    after_line: usize,
    content: Vec<String>,
}

enum DisplayLine<'a> {
    Original(&'a str),
    Overlay(&'a str),
}

impl<'a> DisplayLine<'a> {
    fn text(&self) -> &'a str {
        match self {
            DisplayLine::Original(text) => text,
            DisplayLine::Overlay(text) => text,
        }
    }
}

impl Document {
    fn compose<'a>(&'a self, overlays: &'a [OverlayItem]) -> Vec<DisplayLine<'a>> {
        let mut result = Vec::new();
        let mut o_idx = 0;
        for (i, line) in self.lines.iter().enumerate() {
            result.push(DisplayLine::Original(line));
            while o_idx < overlays.len() && overlays[o_idx].after_line == i {
                for text in &overlays[o_idx].content {
                    result.push(DisplayLine::Overlay(text));
                }
                o_idx += 1;
            }
        }
        while o_idx < overlays.len() {
            for text in &overlays[o_idx].content {
                result.push(DisplayLine::Overlay(text));
            }
            o_idx += 1;
        }
        result
    }
}

struct App {
    doc: Document,
    overlays: Vec<OverlayItem>,
    cursor_x: usize,
    cursor_y: usize,
    scroll: u16,
    mode: Mode,
    search_query: Option<String>,
    search_hits: Vec<(usize, usize)>,
    current_hit: Option<usize>,
    selection_start: Option<(usize, usize)>,
}

impl App {
    fn new(content: String) -> Self {
        Self {
            doc: Document::new(content),
            overlays: Vec::new(),
            cursor_x: 0,
            cursor_y: 0,
            scroll: 0,
            mode: Mode::Normal,
            search_query: None,
            search_hits: Vec::new(),
            current_hit: None,
            selection_start: None,
        }
    }

    fn display_lines(&self) -> Vec<DisplayLine> {
        self.doc.compose(&self.overlays)
    }

    fn line_len(&self, line: usize) -> usize {
        self.display_lines()
            .get(line)
            .map(|l| l.text().len())
            .unwrap_or(0)
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
        if self.cursor_y + 1 < self.display_lines().len() {
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
        self.display_lines()
            .get(y)
            .and_then(|l| l.text().as_bytes().get(x))
            .copied()
    }

    fn char_before(&self, y: usize, x: usize) -> Option<u8> {
        let lines = self.display_lines();
        if x > 0 {
            return lines
                .get(y)
                .and_then(|l| l.text().as_bytes().get(x - 1))
                .copied();
        }
        if y > 0 {
            return lines.get(y - 1)?.text().as_bytes().last().copied();
        }
        None
    }

    fn skip_forward<F>(&self, y: &mut usize, x: &mut usize, pred: F)
    where
        F: Fn(u8) -> bool,
    {
        let lines = self.display_lines();
        while *y < lines.len() {
            let bytes = lines[*y].text().as_bytes();
            while *x < bytes.len() && pred(bytes[*x]) {
                *x += 1;
            }
            if *x < bytes.len() {
                return;
            }
            if *y + 1 == lines.len() {
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
        let lines = self.display_lines();
        loop {
            if *y == 0 && *x == 0 {
                return;
            }
            if *x == 0 {
                *y -= 1;
                *x = lines[*y].text().len();
                if *x == 0 {
                    continue;
                }
            }
            let bytes = lines[*y].text().as_bytes();
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
                self.skip_forward(&mut y, &mut x, is_keyword);
            } else {
                self.skip_forward(&mut y, &mut x, |b| {
                    !is_keyword(b) && !b.is_ascii_whitespace()
                });
            }
        }

        self.skip_forward(&mut y, &mut x, |b| b.is_ascii_whitespace());

        let lines_len = self.display_lines().len();
        self.cursor_y = y.min(lines_len.saturating_sub(1));
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
                self.skip_backward(&mut y, &mut x, is_keyword);
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
        let lines = self.display_lines();
        for (i, line) in lines.iter().enumerate().skip(self.cursor_y + 1) {
            if line.text().trim().is_empty() {
                self.cursor_y = i;
                self.cursor_x = 0;
                return;
            }
        }
        self.cursor_y = lines.len().saturating_sub(1);
        self.cursor_x = 0;
    }

    fn move_paragraph_up(&mut self) {
        if self.cursor_y == 0 {
            return;
        }
        let lines = self.display_lines();
        for (i, line) in lines.iter().enumerate().take(self.cursor_y).rev() {
            if line.text().trim().is_empty() {
                self.cursor_y = i;
                self.cursor_x = 0;
                return;
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
        let mid = (self.scroll + height / 2).min(self.display_lines().len() as u16 - 1);
        self.cursor_y = mid as usize;
        let len = self.line_len(self.cursor_y);
        if self.cursor_x > len {
            self.cursor_x = len;
        }
    }

    fn cursor_bottom(&mut self, height: u16) {
        let bottom = (self.scroll + height - 1).min(self.display_lines().len() as u16 - 1);
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
        let lines = self.display_lines();
        if !lines.is_empty() {
            self.cursor_y = lines.len() - 1;
            self.cursor_x = 0;
        }
    }

    fn set_search_query(&mut self, query: String) {
        if query.is_empty() {
            self.search_query = None;
            self.search_hits.clear();
            self.current_hit = None;
            return;
        }
        self.search_query = Some(query.clone());
        self.search_hits.clear();
        let lines: Vec<String> = self
            .display_lines()
            .into_iter()
            .map(|l| l.text().to_owned())
            .collect();
        for (y, line) in lines.iter().enumerate() {
            let mut start = 0;
            while let Some(pos) = line[start..].find(&query) {
                self.search_hits.push((y, start + pos));
                start += pos + query.len();
            }
        }
        self.current_hit = if self.search_hits.is_empty() {
            None
        } else {
            Some(0)
        };
        if let Some(idx) = self.current_hit {
            let (y, x) = self.search_hits[idx];
            self.cursor_y = y;
            self.cursor_x = x;
        }
    }

    fn clear_search(&mut self) {
        self.search_query = None;
        self.search_hits.clear();
        self.current_hit = None;
    }

    fn next_hit(&mut self, height: u16) {
        if self.search_hits.is_empty() {
            return;
        }
        let next = match self.current_hit {
            Some(i) => (i + 1) % self.search_hits.len(),
            None => 0,
        };
        self.current_hit = Some(next);
        let (y, x) = self.search_hits[next];
        self.cursor_y = y;
        self.cursor_x = x;
        self.ensure_visible(height);
    }

    fn prev_hit(&mut self, height: u16) {
        if self.search_hits.is_empty() {
            return;
        }
        let prev = match self.current_hit {
            Some(0) | None => self.search_hits.len() - 1,
            Some(i) => i - 1,
        };
        self.current_hit = Some(prev);
        let (y, x) = self.search_hits[prev];
        self.cursor_y = y;
        self.cursor_x = x;
        self.ensure_visible(height);
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
            let height = terminal.size()?.height.saturating_sub(1);

            let mode = app.mode.clone();
            let quit = match mode {
                Mode::Normal => {
                    app.mode = Mode::Normal;
                    keymaps::normal::handle(&mut app, key, height, &mut pending_g)
                }
                Mode::Visual => {
                    app.mode = Mode::Visual;
                    keymaps::visual::handle(&mut app, key, height)
                }
                Mode::VisualLine => {
                    app.mode = Mode::VisualLine;
                    keymaps::visual::handle(&mut app, key, height)
                }
                Mode::Command(mut cmd) => {
                    let quit = keymaps::command::handle(&mut app, &mut cmd, key, height);
                    if matches!(app.mode, Mode::Command(_)) {
                        app.mode = Mode::Command(cmd);
                    }
                    quit
                }
                Mode::Search(mut query) => {
                    let quit = keymaps::search::handle(&mut app, &mut query, key, height);
                    if matches!(app.mode, Mode::Search(_)) {
                        app.mode = Mode::Search(query);
                    }
                    quit
                }
                Mode::Help => {
                    app.mode = Mode::Help;
                    keymaps::help::handle(&mut app, key, height)
                }
            };

            if quit {
                return Ok(());
            }
        }
    }
}

fn ui(f: &mut Frame, app: &App) {
    let area = f.area();
    if matches!(app.mode, Mode::Help) {
        let paragraph = Paragraph::new(HELP_TEXT).wrap(Wrap { trim: true });
        f.render_widget(paragraph, area);
        return;
    }
    let main_height = area.height.saturating_sub(1);
    let main_area = Rect {
        x: area.x,
        y: area.y,
        width: area.width,
        height: main_height,
    };
    let selection = app
        .selection_start
        .map(|s| (s, (app.cursor_y, app.cursor_x)));
    let line_mode = matches!(app.mode, Mode::VisualLine);
    let lines: Vec<Line> = app
        .display_lines()
        .iter()
        .enumerate()
        .map(|(i, l)| {
            highlight_line(
                l.text(),
                i,
                app.search_query.as_deref(),
                selection,
                line_mode,
            )
        })
        .collect();
    let text = Text::from(lines);
    let paragraph = Paragraph::new(text)
        .wrap(Wrap { trim: true })
        .scroll((app.scroll, 0));
    f.render_widget(paragraph, main_area);
    let cursor_y = main_area.y + (app.cursor_y as u16).saturating_sub(app.scroll);
    let cursor_x = main_area.x + app.cursor_x as u16;
    f.set_cursor_position((cursor_x, cursor_y));

    let cmd_area = Rect {
        x: area.x,
        y: area.y + main_height,
        width: area.width,
        height: 1,
    };

    match &app.mode {
        Mode::Command(cmd) => {
            let text = format!(":{}", cmd);
            let paragraph = Paragraph::new(text);
            f.render_widget(paragraph, cmd_area);
            f.set_cursor_position((cmd_area.x + 1 + cmd.len() as u16, cmd_area.y));
        }
        Mode::Search(query) => {
            let text = format!("/{}", query);
            let paragraph = Paragraph::new(text);
            f.render_widget(paragraph, cmd_area);
            f.set_cursor_position((cmd_area.x + 1 + query.len() as u16, cmd_area.y));
        }
        _ => {
            let blank = Paragraph::new("");
            f.render_widget(blank, cmd_area);
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use insta::assert_snapshot;
    use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
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
        let content: String = (1..=20).map(|i| format!("line {i}\n")).collect();
        let mut app = App::new(content);
        let backend = TestBackend::new(20, 5);
        let mut terminal = Terminal::new(backend).unwrap();
        let height = terminal.size().unwrap().height.saturating_sub(1);

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

    #[test]
    fn command_q_ui() {
        let content = "hello\nworld".to_string();
        let mut app = App::new(content);
        app.mode = Mode::Command("q".into());
        let backend = TestBackend::new(20, 5);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| ui(f, &app)).unwrap();
        assert_snapshot!("command_q_ui", terminal.backend());
    }

    #[test]
    fn colon_enters_command_mode() {
        let content = "hello".to_string();
        let mut app = App::new(content);
        let backend = TestBackend::new(20, 5);
        let mut terminal = Terminal::new(backend).unwrap();
        let height = terminal.size().unwrap().height.saturating_sub(1);
        let mut pending_g = false;
        let key = KeyEvent::new(KeyCode::Char(':'), KeyModifiers::NONE);
        keymaps::normal::handle(&mut app, key, height, &mut pending_g);
        terminal.draw(|f| ui(f, &app)).unwrap();
        assert_snapshot!("colon_enters_command_mode", terminal.backend());
    }

    #[test]
    fn slash_enters_search_mode() {
        let content = "hello".to_string();
        let mut app = App::new(content);
        let backend = TestBackend::new(20, 5);
        let mut terminal = Terminal::new(backend).unwrap();
        let height = terminal.size().unwrap().height.saturating_sub(1);
        let mut pending_g = false;
        let key = KeyEvent::new(KeyCode::Char('/'), KeyModifiers::NONE);
        keymaps::normal::handle(&mut app, key, height, &mut pending_g);
        terminal.draw(|f| ui(f, &app)).unwrap();
        assert_snapshot!("slash_enters_search_mode", terminal.backend());
    }
    #[test]
    fn help_screen_renders() {
        let content = "hello".to_string();
        let mut app = App::new(content);
        app.mode = Mode::Help;
        let backend = TestBackend::new(20, 5);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| ui(f, &app)).unwrap();
        assert_snapshot!("help_screen", terminal.backend());
    }
}
