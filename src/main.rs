use anyhow::Result;
use clap::{Parser, CommandFactory, ColorChoice};
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

#[derive(Parser, Debug)]
#[command(author, version, about = "View files in a terminal UI")] 
struct Cli {
    /// Display file without the interactive UI
    #[arg(long)]
    headless: bool,

    /// File to display
    file: Option<PathBuf>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let Some(path) = cli.file.clone() else {
        Cli::command().color(ColorChoice::Never).print_help()?;
        println!();
        return Ok(());
    };
    let headless = cli.headless;
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
    loop {
        terminal.draw(|f| ui(f, &content))?;

        if let Event::Key(key) = event::read()? {
            if key.code == KeyCode::Char('q') {
                return Ok(());
            }
        }
    }
}

fn ui(f: &mut Frame, content: &str) {
    let area = f.area();
    let paragraph = Paragraph::new(content).wrap(Wrap { trim: true });
    f.render_widget(paragraph, area);
}