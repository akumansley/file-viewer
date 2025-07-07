use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::{App, Mode};

#[derive(Clone, Copy)]
pub enum EditorCommand {
    Quit,
    EnterVisual,
    EnterVisualLine,
    GotoFirstOrPending,
    EnterHelp,
    GotoLastLine,
    EnterSearch,
    NextHit,
    PrevHit,
    EnterCommand,
    MoveLeft,
    MoveDown,
    MoveUp,
    MoveRight,
    MoveWordForward,
    MoveWordBackward,
    MoveParagraphUp,
    MoveParagraphDown,
    HalfPageUp,
    HalfPageDown,
    CursorTop,
    CursorMiddle,
    CursorBottom,
    CancelSelection,
    ExitHelp,
    ExitCommand,
    CommandSubmit,
    CommandBackspace,
    CommandChar(char),
    ExitSearch,
    ClearSearch,
    SearchSubmit,
    SearchBackspace,
    SearchChar(char),
}

pub struct Context {
    pub height: u16,
    pub pending_g: bool,
}

impl EditorCommand {
    pub fn run(self, app: &mut App, ctx: &mut Context) -> bool {
        match self {
            EditorCommand::Quit => return true,
            EditorCommand::EnterVisual => {
                app.mode = Mode::Visual;
                app.selection_start = Some((app.cursor_y, app.cursor_x));
                ctx.pending_g = false;
            }
            EditorCommand::EnterVisualLine => {
                app.mode = Mode::VisualLine;
                app.selection_start = Some((app.cursor_y, app.cursor_x));
                ctx.pending_g = false;
            }
            EditorCommand::GotoFirstOrPending => {
                if ctx.pending_g {
                    app.goto_first_line();
                    app.ensure_visible(ctx.height);
                    ctx.pending_g = false;
                } else {
                    ctx.pending_g = true;
                }
            }
            EditorCommand::EnterHelp => {
                app.mode = Mode::Help;
                ctx.pending_g = false;
            }
            EditorCommand::GotoLastLine => {
                app.goto_last_line();
                app.ensure_visible(ctx.height);
                ctx.pending_g = false;
            }
            EditorCommand::EnterSearch => {
                app.mode = Mode::Search(String::new());
                ctx.pending_g = false;
            }
            EditorCommand::NextHit => app.next_hit(ctx.height),
            EditorCommand::PrevHit => app.prev_hit(ctx.height),
            EditorCommand::EnterCommand => {
                app.mode = Mode::Command(String::new());
                ctx.pending_g = false;
            }
            EditorCommand::MoveLeft => app.move_left(),
            EditorCommand::MoveDown => app.move_down(ctx.height),
            EditorCommand::MoveUp => app.move_up(),
            EditorCommand::MoveRight => app.move_right(),
            EditorCommand::MoveWordForward => {
                app.move_word_forward();
                app.ensure_visible(ctx.height);
            }
            EditorCommand::MoveWordBackward => {
                app.move_word_backward();
                app.ensure_visible(ctx.height);
            }
            EditorCommand::MoveParagraphUp => {
                app.move_paragraph_up();
                app.ensure_visible(ctx.height);
            }
            EditorCommand::MoveParagraphDown => {
                app.move_paragraph_down();
                app.ensure_visible(ctx.height);
            }
            EditorCommand::HalfPageUp => app.half_page_up(ctx.height),
            EditorCommand::HalfPageDown => app.half_page_down(ctx.height),
            EditorCommand::CursorTop => {
                app.cursor_top();
                app.ensure_visible(ctx.height);
            }
            EditorCommand::CursorMiddle => {
                app.cursor_middle(ctx.height);
                app.ensure_visible(ctx.height);
            }
            EditorCommand::CursorBottom => {
                app.cursor_bottom(ctx.height);
                app.ensure_visible(ctx.height);
            }
            EditorCommand::CancelSelection => {
                app.mode = Mode::Normal;
                app.selection_start = None;
            }
            EditorCommand::ExitHelp => app.mode = Mode::Normal,
            EditorCommand::ExitCommand => app.mode = Mode::Normal,
            EditorCommand::CommandSubmit => {
                let cmd = if let Mode::Command(ref mut c) = app.mode {
                    c.trim().to_string()
                } else {
                    String::new()
                };
                match cmd.as_str() {
                    "q" => return true,
                    "help" => app.mode = Mode::Help,
                    _ => {
                        let mut parts = cmd.splitn(2, char::is_whitespace);
                        let name = parts.next().unwrap_or("");
                        let args = parts.next().unwrap_or("");
                        if let Some(spec) = app.commands.get(name) {
                            let mut template = spec.template.clone();
                            template = template.replace("{line}", &(app.cursor_y + 1).to_string());
                            template = template.replace("{col}", &(app.cursor_x + 1).to_string());
                            template = template.replace("{args}", args);
                            let ((sy, sx), (ey, ex)) = match app.selection_start {
                                Some(start) => {
                                    let end = (app.cursor_y, app.cursor_x);
                                    if start <= end {
                                        (start, end)
                                    } else {
                                        (end, start)
                                    }
                                }
                                None => {
                                    let pos = (app.cursor_y, app.cursor_x);
                                    (pos, pos)
                                }
                            };
                            template = template.replace("{start_line}", &(sy + 1).to_string());
                            template = template.replace("{start_col}", &(sx + 1).to_string());
                            template = template.replace("{end_line}", &(ey + 1).to_string());
                            template = template.replace("{end_col}", &(ex + 1).to_string());

                            let parts: Vec<&str> = template.split_whitespace().collect();
                            if let Some((prog, rest)) = parts.split_first() {
                                let mut command = std::process::Command::new(prog);
                                command.args(rest);
                                let _ = command.status();
                            }
                        }
                        app.mode = Mode::Normal;
                    }
                }
            }
            EditorCommand::CommandBackspace => {
                if let Mode::Command(ref mut c) = app.mode {
                    c.pop();
                }
            }
            EditorCommand::CommandChar(ch) => {
                if let Mode::Command(ref mut c) = app.mode {
                    c.push(ch);
                }
            }
            EditorCommand::ExitSearch => app.mode = Mode::Normal,
            EditorCommand::ClearSearch => {
                app.clear_search();
                app.mode = Mode::Normal;
            }
            EditorCommand::SearchSubmit => {
                if let Mode::Search(ref mut q) = app.mode {
                    let q2 = q.clone();
                    app.set_search_query(q2);
                }
                app.mode = Mode::Normal;
                app.ensure_visible(ctx.height);
            }
            EditorCommand::SearchBackspace => {
                if let Mode::Search(ref mut q) = app.mode {
                    q.pop();
                }
            }
            EditorCommand::SearchChar(ch) => {
                if let Mode::Search(ref mut q) = app.mode {
                    q.push(ch);
                }
            }
        }
        false
    }
}

pub struct KeyBinding {
    pub key: KeyEvent,
    pub command: EditorCommand,
    pub help: &'static str,
}

pub const NORMAL_BINDINGS: &[KeyBinding] = &[
    KeyBinding {
        key: KeyEvent::new(KeyCode::Char('v'), KeyModifiers::NONE),
        command: EditorCommand::EnterVisual,
        help: "Start visual mode",
    },
    KeyBinding {
        key: KeyEvent::new(KeyCode::Char('V'), KeyModifiers::NONE),
        command: EditorCommand::EnterVisualLine,
        help: "Start visual line mode",
    },
    KeyBinding {
        key: KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE),
        command: EditorCommand::GotoFirstOrPending,
        help: "gg goto first line",
    },
    KeyBinding {
        key: KeyEvent::new(KeyCode::Char('?'), KeyModifiers::NONE),
        command: EditorCommand::EnterHelp,
        help: "Show this help",
    },
    KeyBinding {
        key: KeyEvent::new(KeyCode::Char('G'), KeyModifiers::NONE),
        command: EditorCommand::GotoLastLine,
        help: "Goto last line",
    },
    KeyBinding {
        key: KeyEvent::new(KeyCode::Char('/'), KeyModifiers::NONE),
        command: EditorCommand::EnterSearch,
        help: "Search",
    },
    KeyBinding {
        key: KeyEvent::new(KeyCode::Char('n'), KeyModifiers::NONE),
        command: EditorCommand::NextHit,
        help: "Next search hit",
    },
    KeyBinding {
        key: KeyEvent::new(KeyCode::Char('N'), KeyModifiers::NONE),
        command: EditorCommand::PrevHit,
        help: "Prev search hit",
    },
    KeyBinding {
        key: KeyEvent::new(KeyCode::Char(':'), KeyModifiers::NONE),
        command: EditorCommand::EnterCommand,
        help: "Command mode",
    },
    KeyBinding {
        key: KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE),
        command: EditorCommand::Quit,
        help: "Quit",
    },
    KeyBinding {
        key: KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE),
        command: EditorCommand::MoveLeft,
        help: "Move left",
    },
    KeyBinding {
        key: KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE),
        command: EditorCommand::MoveDown,
        help: "Move down",
    },
    KeyBinding {
        key: KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE),
        command: EditorCommand::MoveUp,
        help: "Move up",
    },
    KeyBinding {
        key: KeyEvent::new(KeyCode::Char('l'), KeyModifiers::NONE),
        command: EditorCommand::MoveRight,
        help: "Move right",
    },
    KeyBinding {
        key: KeyEvent::new(KeyCode::Char('w'), KeyModifiers::NONE),
        command: EditorCommand::MoveWordForward,
        help: "Next word",
    },
    KeyBinding {
        key: KeyEvent::new(KeyCode::Char('b'), KeyModifiers::NONE),
        command: EditorCommand::MoveWordBackward,
        help: "Prev word",
    },
    KeyBinding {
        key: KeyEvent::new(KeyCode::Char('{'), KeyModifiers::NONE),
        command: EditorCommand::MoveParagraphUp,
        help: "Prev paragraph",
    },
    KeyBinding {
        key: KeyEvent::new(KeyCode::Char('}'), KeyModifiers::NONE),
        command: EditorCommand::MoveParagraphDown,
        help: "Next paragraph",
    },
    KeyBinding {
        key: KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL),
        command: EditorCommand::HalfPageUp,
        help: "Half page up",
    },
    KeyBinding {
        key: KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL),
        command: EditorCommand::HalfPageDown,
        help: "Half page down",
    },
    KeyBinding {
        key: KeyEvent::new(KeyCode::Char('H'), KeyModifiers::NONE),
        command: EditorCommand::CursorTop,
        help: "Top of screen",
    },
    KeyBinding {
        key: KeyEvent::new(KeyCode::Char('M'), KeyModifiers::NONE),
        command: EditorCommand::CursorMiddle,
        help: "Middle of screen",
    },
    KeyBinding {
        key: KeyEvent::new(KeyCode::Char('L'), KeyModifiers::NONE),
        command: EditorCommand::CursorBottom,
        help: "Bottom of screen",
    },
];

pub const VISUAL_BINDINGS: &[KeyBinding] = &[
    KeyBinding {
        key: KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL),
        command: EditorCommand::CancelSelection,
        help: "Cancel selection",
    },
    KeyBinding {
        key: KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE),
        command: EditorCommand::Quit,
        help: "Quit",
    },
    KeyBinding {
        key: KeyEvent::new(KeyCode::Char('?'), KeyModifiers::NONE),
        command: EditorCommand::EnterHelp,
        help: "Show this help",
    },
    KeyBinding {
        key: KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE),
        command: EditorCommand::MoveLeft,
        help: "Move left",
    },
    KeyBinding {
        key: KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE),
        command: EditorCommand::MoveDown,
        help: "Move down",
    },
    KeyBinding {
        key: KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE),
        command: EditorCommand::MoveUp,
        help: "Move up",
    },
    KeyBinding {
        key: KeyEvent::new(KeyCode::Char('l'), KeyModifiers::NONE),
        command: EditorCommand::MoveRight,
        help: "Move right",
    },
    KeyBinding {
        key: KeyEvent::new(KeyCode::Char('w'), KeyModifiers::NONE),
        command: EditorCommand::MoveWordForward,
        help: "Next word",
    },
    KeyBinding {
        key: KeyEvent::new(KeyCode::Char('b'), KeyModifiers::NONE),
        command: EditorCommand::MoveWordBackward,
        help: "Prev word",
    },
    KeyBinding {
        key: KeyEvent::new(KeyCode::Char('{'), KeyModifiers::NONE),
        command: EditorCommand::MoveParagraphUp,
        help: "Prev paragraph",
    },
    KeyBinding {
        key: KeyEvent::new(KeyCode::Char('}'), KeyModifiers::NONE),
        command: EditorCommand::MoveParagraphDown,
        help: "Next paragraph",
    },
    KeyBinding {
        key: KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL),
        command: EditorCommand::HalfPageUp,
        help: "Half page up",
    },
    KeyBinding {
        key: KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL),
        command: EditorCommand::HalfPageDown,
        help: "Half page down",
    },
    KeyBinding {
        key: KeyEvent::new(KeyCode::Char('H'), KeyModifiers::NONE),
        command: EditorCommand::CursorTop,
        help: "Top of screen",
    },
    KeyBinding {
        key: KeyEvent::new(KeyCode::Char('M'), KeyModifiers::NONE),
        command: EditorCommand::CursorMiddle,
        help: "Middle of screen",
    },
    KeyBinding {
        key: KeyEvent::new(KeyCode::Char('L'), KeyModifiers::NONE),
        command: EditorCommand::CursorBottom,
        help: "Bottom of screen",
    },
];

pub const COMMAND_BINDINGS: &[KeyBinding] = &[
    KeyBinding {
        key: KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE),
        command: EditorCommand::ExitCommand,
        help: "Exit command",
    },
    KeyBinding {
        key: KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL),
        command: EditorCommand::ExitCommand,
        help: "Exit command",
    },
    KeyBinding {
        key: KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
        command: EditorCommand::CommandSubmit,
        help: "Execute command",
    },
    KeyBinding {
        key: KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE),
        command: EditorCommand::CommandBackspace,
        help: "Delete char",
    },
];

pub const SEARCH_BINDINGS: &[KeyBinding] = &[
    KeyBinding {
        key: KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL),
        command: EditorCommand::ClearSearch,
        help: "Cancel search",
    },
    KeyBinding {
        key: KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE),
        command: EditorCommand::ExitSearch,
        help: "Exit search",
    },
    KeyBinding {
        key: KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
        command: EditorCommand::SearchSubmit,
        help: "Search",
    },
    KeyBinding {
        key: KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE),
        command: EditorCommand::SearchBackspace,
        help: "Delete char",
    },
];

pub const HELP_BINDINGS: &[KeyBinding] = &[
    KeyBinding {
        key: KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE),
        command: EditorCommand::ExitHelp,
        help: "Close help",
    },
    KeyBinding {
        key: KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE),
        command: EditorCommand::ExitHelp,
        help: "Close help",
    },
];

pub fn lookup_and_run(
    bindings: &[KeyBinding],
    key: KeyEvent,
    app: &mut App,
    ctx: &mut Context,
) -> bool {
    for b in bindings {
        if b.key == key {
            return b.command.run(app, ctx);
        }
    }
    ctx.pending_g = false;
    false
}

fn format_key(key: KeyEvent) -> String {
    use KeyCode::*;
    let mut parts = Vec::new();
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        parts.push("Ctrl".to_string());
    }
    if key.modifiers.contains(KeyModifiers::ALT) {
        parts.push("Alt".to_string());
    }
    let code = match key.code {
        Char(c) => c.to_string(),
        Enter => "Enter".to_string(),
        Esc => "Esc".to_string(),
        Backspace => "Backspace".to_string(),
        _ => format!("{:?}", key.code),
    };
    parts.push(code);
    parts.join("-")
}

pub fn help_lines() -> Vec<String> {
    let mut lines = vec!["File Viewer Help".to_string(), String::new()];

    lines.push("Normal mode:".to_string());
    for binding in NORMAL_BINDINGS {
        lines.push(format!("{} - {}", format_key(binding.key), binding.help));
    }
    lines.push(String::new());

    lines.push("Visual mode:".to_string());
    for binding in VISUAL_BINDINGS {
        lines.push(format!("{} - {}", format_key(binding.key), binding.help));
    }
    lines.push(String::new());

    lines.push("Command mode:".to_string());
    for binding in COMMAND_BINDINGS {
        lines.push(format!("{} - {}", format_key(binding.key), binding.help));
    }
    lines.push(String::new());

    lines.push("Search mode:".to_string());
    for binding in SEARCH_BINDINGS {
        lines.push(format!("{} - {}", format_key(binding.key), binding.help));
    }
    lines.push(String::new());

    lines.push("Help screen:".to_string());
    for binding in HELP_BINDINGS {
        lines.push(format!("{} - {}", format_key(binding.key), binding.help));
    }

    lines
}
