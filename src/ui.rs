use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode},
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
use std::io::Stdout;

use std::io::stdout;

use crate::{
    scoring::score_and_sort,
    solver::{Feedback, SolverState, parse_pattern},
    wordlist::load_words,
};

pub struct App {
    words: Vec<String>,
    solver: SolverState,
    input: String,
    suggestions: Vec<(String, usize)>,
}

impl App {
    pub fn new(words: Vec<String>, word_len: usize) -> Self {
        Self {
            solver: SolverState::new(word_len),
            words,
            input: String::new(),
            suggestions: Vec::new(),
        }
    }

    pub fn run(&mut self, terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
        loop {
            terminal.draw(|f| self.draw(f))?;

            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => return Ok(()),

                    KeyCode::Enter => {
                        self.submit_input()?;
                    }

                    KeyCode::Backspace => {
                        self.input.pop();
                    }

                    KeyCode::Char(c) => {
                        self.input.push(c);
                    }

                    _ => {}
                }
            }
        }
    }

    fn submit_input(&mut self) -> Result<()> {
        let parts: Vec<_> = self.input.split_whitespace().collect();
        if parts.len() != 2 {
            self.input.clear();
            return Ok(());
        }

        let guess = parts[0].to_lowercase();
        let feedback = parse_pattern(parts[1])?;

        self.solver.add_guess(guess, feedback);
        self.recompute();

        self.input.clear();
        Ok(())
    }

    fn recompute(&mut self) {
        let remaining = self.solver.filter(&self.words);
        self.suggestions = score_and_sort(&remaining);
    }

    fn draw(&self, f: &mut Frame) {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(8),
                Constraint::Min(5),
                Constraint::Length(3),
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
                let mut spans = Vec::new();

                for (c, fb) in g.word.chars().zip(g.feedback.iter()) {
                    let style = match fb {
                        Feedback::Green => Style::default().bg(Color::Green).fg(Color::Black),
                        Feedback::Yellow => Style::default().bg(Color::Yellow).fg(Color::Black),
                        Feedback::Gray => Style::default().bg(Color::DarkGray).fg(Color::White),
                    };

                    spans.push(Span::styled(format!(" {} ", c.to_ascii_uppercase()), style));
                }

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
        let text = format!("{}▌", self.input);

        f.render_widget(
            Paragraph::new(text).block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Input: <guess> <pattern> | q to quit"),
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
