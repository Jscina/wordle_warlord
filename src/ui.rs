use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
};
use std::io::{Stdout, stdout};

use crate::{
    scoring::score_and_sort,
    solver::{Feedback, SolverState, parse_pattern},
    wordlist::load_words,
};

enum InputStatus {
    Incomplete,
    Invalid(&'static str),
    Valid,
}

enum ParsedInput {
    Incomplete,
    Invalid,
    Valid {
        guess: String,
        feedback: Vec<Feedback>,
    },
}

pub struct App {
    words: Vec<String>,
    solver: SolverState,
    input: String,
    suggestions: Vec<(String, usize)>,
}

impl App {
    pub fn new(words: Vec<String>, word_len: usize) -> Self {
        Self {
            words,
            solver: SolverState::new(word_len),
            input: String::new(),
            suggestions: Vec::new(),
        }
    }

    pub fn run(&mut self, terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
        loop {
            terminal.draw(|f| self.draw(f))?;

            let event = event::read()?;
            if let Event::Key(key) = event {
                if self.handle_key(key) {
                    return Ok(());
                }
            }
        }
    }

    /// Returns true if the app should exit
    fn handle_key(&mut self, key: event::KeyEvent) -> bool {
        match (key.code, key.modifiers) {
            // Quit
            (KeyCode::Char('q'), KeyModifiers::CONTROL) => return true,

            // Undo
            (KeyCode::Char('z'), KeyModifiers::CONTROL) => {
                self.undo_last_guess();
            }

            // Submit
            (KeyCode::Enter, _) => {
                self.submit_input();
            }

            // Backspace
            (KeyCode::Backspace, _) => {
                self.input.pop();
            }

            // Normal text input — ALWAYS allowed
            (KeyCode::Char(c), _) => {
                self.input.push(c);
            }

            _ => {}
        }

        false
    }

    fn parse_input(&self) -> ParsedInput {
        let parts: Vec<_> = self.input.split_whitespace().collect();
        if parts.len() != 2 {
            return ParsedInput::Incomplete;
        }

        let guess = parts[0].to_lowercase();
        let pattern = parts[1];

        if guess.len() != self.solver.word_len() {
            return ParsedInput::Invalid;
        }

        if pattern.len() != self.solver.word_len() {
            return ParsedInput::Invalid;
        }

        let feedback = match parse_pattern(pattern) {
            Ok(f) => f,
            Err(_) => return ParsedInput::Invalid,
        };

        ParsedInput::Valid { guess, feedback }
    }

    fn input_status(&self) -> InputStatus {
        let parts: Vec<_> = self.input.split_whitespace().collect();

        if parts.is_empty() {
            return InputStatus::Incomplete;
        }

        if parts.len() == 1 {
            return InputStatus::Incomplete;
        }

        if parts.len() > 2 {
            return InputStatus::Invalid("too many fields");
        }

        let guess = parts[0];
        let pattern = parts[1];

        if guess.len() != self.solver.word_len() {
            return InputStatus::Invalid("guess length mismatch");
        }

        if pattern.len() != self.solver.word_len() {
            return InputStatus::Invalid("pattern length mismatch");
        }

        if parse_pattern(pattern).is_err() {
            return InputStatus::Invalid("pattern must be G/Y/X");
        }

        InputStatus::Valid
    }

    fn submit_input(&mut self) {
        if !matches!(self.input_status(), InputStatus::Valid) {
            return;
        }

        if let ParsedInput::Valid { guess, feedback } = self.parse_input() {
            self.solver.add_guess(guess, feedback);
            self.recompute();
            self.input.clear();
        }
    }
    fn recompute(&mut self) {
        let remaining = self.solver.filter(&self.words);
        self.suggestions = score_and_sort(&remaining);
    }

    fn undo_last_guess(&mut self) {
        if !self.solver.guesses().is_empty() {
            self.solver.pop_guess();
            self.recompute();
        }
    }

    fn draw(&self, f: &mut Frame) {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(8), // guesses
                Constraint::Min(5),    // suggestions
                Constraint::Length(3), // input
            ])
            .split(f.area());

        self.draw_guesses(f, layout[0]);
        self.draw_suggestions(f, layout[1]);
        self.draw_input(f, layout[2]);
    }

    fn draw_guesses(&self, f: &mut Frame, area: ratatui::layout::Rect) {
        let lines: Vec<Line> = self
            .solver
            .guesses()
            .iter()
            .map(|g| {
                let spans: Vec<Span> = g
                    .word
                    .chars()
                    .zip(g.feedback.iter())
                    .map(|(c, fb)| {
                        let style = match fb {
                            Feedback::Green => Style::default().bg(Color::Green).fg(Color::Black),
                            Feedback::Yellow => Style::default().bg(Color::Yellow).fg(Color::Black),
                            Feedback::Gray => Style::default().bg(Color::DarkGray).fg(Color::White),
                        };
                        Span::styled(format!(" {} ", c.to_ascii_uppercase()), style)
                    })
                    .collect();
                Line::from(spans)
            })
            .collect();

        f.render_widget(
            Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title("Guesses")),
            area,
        );
    }

    fn draw_suggestions(&self, f: &mut Frame, area: ratatui::layout::Rect) {
        let items: Vec<ListItem> = self
            .suggestions
            .iter()
            .take(10)
            .map(|(w, s)| ListItem::new(format!("{w} ({s})")))
            .collect();

        let title = format!("Suggestions (remaining: {})", self.suggestions.len());

        f.render_widget(
            List::new(items).block(Block::default().borders(Borders::ALL).title(title)),
            area,
        );
    }

    fn draw_input(&self, f: &mut Frame, area: ratatui::layout::Rect) {
        let status = self.input_status();

        let (border_color, subtitle) = match status {
            InputStatus::Incomplete => (Color::Gray, ""),
            InputStatus::Valid => (Color::Green, ""),
            InputStatus::Invalid(msg) => (Color::Red, msg),
        };

        let text = format!("{}▌", self.input);

        f.render_widget(
            Paragraph::new(text).block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(border_color))
                    .title(format!(
                        "Input {} | Enter = submit | Ctrl+Z = undo | Ctrl+Q = quit",
                        subtitle
                    )),
            ),
            area,
        );
    }
}

pub fn run_ui() -> Result<()> {
    let words = load_words()?;
    let mut app = App::new(words, 5);

    let mut stdout = stdout();
    enable_raw_mode()?;
    execute!(stdout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = app.run(&mut terminal);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

