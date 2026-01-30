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
    analysis::{
        ConstraintSummary, LetterAnalysis, PositionAnalysis, SolutionPoolStats,
        compute_constraint_summary, compute_letter_analysis, compute_position_analysis,
        compute_solution_pool_stats,
    },
    scoring::score_and_sort,
    solver::{Feedback, Guess, SolverState, generate_feedback, parse_pattern},
    wordlist::{load_words, select_random_word},
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
        word: String,
        feedback: Vec<Feedback>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum GameMode {
    Solver,
    Game,
}

pub struct App {
    words: Vec<String>,
    solver: SolverState,
    input: String,
    suggestions: Vec<(String, usize)>,
    mode: GameMode,
    target_word: Option<String>,
    remaining_guesses: usize,
    game_won: bool,
    game_over: bool,
    // Analysis data (cached)
    letter_analysis: Option<LetterAnalysis>,
    position_analysis: Option<PositionAnalysis>,
    constraint_summary: Option<ConstraintSummary>,
    solution_pool_stats: Option<SolutionPoolStats>,
    // Caching flag
    analysis_dirty: bool,
}

impl App {
    pub fn new(words: Vec<String>, word_len: usize) -> Self {
        Self {
            words,
            solver: SolverState::new(word_len),
            input: String::new(),
            suggestions: Vec::new(),
            mode: GameMode::Solver,
            target_word: None,
            remaining_guesses: 6,
            game_won: false,
            game_over: false,
            letter_analysis: None,
            position_analysis: None,
            constraint_summary: None,
            solution_pool_stats: None,
            analysis_dirty: true,
        }
    }

    pub fn run(&mut self, terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
        loop {
            // Recompute analysis if needed before drawing
            self.recompute_analysis();

            terminal.draw(|f| self.draw(f))?;

            let event = event::read()?;
            if let Event::Key(key) = event
                && self.handle_key(key)
            {
                return Ok(());
            }
        }
    }

    /// Returns true if the app should exit
    fn handle_key(&mut self, key: event::KeyEvent) -> bool {
        match (key.code, key.modifiers) {
            // Quit
            (KeyCode::Char('q'), KeyModifiers::CONTROL) => return true,

            // Toggle Game Mode
            (KeyCode::Char('g'), KeyModifiers::CONTROL) => {
                self.toggle_game_mode();
            }

            // Toggle Solver Mode
            (KeyCode::Char('s'), KeyModifiers::CONTROL) => {
                if self.mode == GameMode::Game {
                    self.mode = GameMode::Solver;
                    self.recompute();
                }
            }

            // Undo
            (KeyCode::Char('z'), KeyModifiers::CONTROL) => {
                if self.mode == GameMode::Solver {
                    self.undo_last_guess();
                } else {
                    self.undo_game_guess();
                }
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
        if self.mode == GameMode::Game {
            let word = self.input.trim().to_lowercase();
            if word.len() != self.solver.word_len() {
                return ParsedInput::Invalid;
            }
            // In game mode, we don't parse pattern - it's generated
            return ParsedInput::Valid {
                word,
                feedback: Vec::new(),
            };
        }

        let parts: Vec<_> = self.input.split_whitespace().collect();
        if parts.len() != 2 {
            return ParsedInput::Incomplete;
        }

        let word = parts[0].to_lowercase();
        let pattern = parts[1];

        if word.len() != self.solver.word_len() {
            return ParsedInput::Invalid;
        }

        if pattern.len() != self.solver.word_len() {
            return ParsedInput::Invalid;
        }

        let feedback = match parse_pattern(pattern) {
            Ok(f) => f,
            Err(_) => return ParsedInput::Invalid,
        };

        ParsedInput::Valid { word, feedback }
    }

    fn input_status(&self) -> InputStatus {
        if self.mode == GameMode::Game {
            let guess = self.input.trim();
            if guess.is_empty() {
                return InputStatus::Incomplete;
            }
            if guess.len() != self.solver.word_len() {
                return InputStatus::Invalid("guess length mismatch");
            }
            return InputStatus::Valid;
        }

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
        if self.mode == GameMode::Game && self.game_over {
            self.start_new_game();
            return;
        }

        if !matches!(self.input_status(), InputStatus::Valid) {
            return;
        }

        if self.mode == GameMode::Game {
            if let Some(ref target) = self.target_word {
                let word = self.input.trim().to_lowercase();
                let feedback = generate_feedback(target, &word);

                self.solver.add_guess(Guess::new(word, feedback.clone()));

                self.remaining_guesses -= 1;
                self.check_game_state(&feedback);

                self.recompute();
                self.input.clear();
            }
        } else if let ParsedInput::Valid { word, feedback } = self.parse_input() {
            let guess = Guess::new(word, feedback.clone());
            self.solver.add_guess(guess);

            self.recompute();
            self.input.clear();
        }
    }

    fn recompute(&mut self) {
        let remaining = self.solver.filter(&self.words);

        if self.solver.guesses().is_empty() {
            self.suggestions.clear();
        } else {
            self.suggestions = score_and_sort(&remaining);
        }

        self.analysis_dirty = true;
    }

    fn recompute_analysis(&mut self) {
        if !self.analysis_dirty {
            return;
        }

        let remaining = self.solver.filter(&self.words);

        self.letter_analysis = Some(compute_letter_analysis(&remaining));
        self.position_analysis = Some(compute_position_analysis(&remaining, &self.solver));
        self.constraint_summary = Some(compute_constraint_summary(&self.solver));
        self.solution_pool_stats = Some(compute_solution_pool_stats(&self.words, &remaining));

        self.analysis_dirty = false;
    }

    fn undo_last_guess(&mut self) {
        if !self.solver.guesses().is_empty() {
            self.solver.pop_guess();
            self.recompute();
            self.analysis_dirty = true;
        }
    }

    fn toggle_game_mode(&mut self) {
        if self.mode == GameMode::Solver {
            self.start_new_game();
        } else {
            self.mode = GameMode::Solver;
            self.recompute();
            self.analysis_dirty = true;
        }
    }

    fn start_new_game(&mut self) {
        match select_random_word(&self.words, self.solver.word_len()) {
            Ok(target) => {
                self.mode = GameMode::Game;
                self.target_word = Some(target);
                self.remaining_guesses = 6;
                self.game_won = false;
                self.game_over = false;
                self.solver = SolverState::new(self.solver.word_len());
                self.input.clear();
                self.recompute();
                self.analysis_dirty = true;
            }
            Err(_) => {
                // Handle error silently for now
            }
        }
    }

    fn undo_game_guess(&mut self) {
        if !self.solver.guesses().is_empty() {
            self.solver.pop_guess();
            self.remaining_guesses += 1;
            self.game_won = false;
            self.game_over = false;
            self.recompute();
            self.analysis_dirty = true;
        }
    }

    fn check_game_state(&mut self, feedback: &[Feedback]) {
        // Check if won (all green)
        if feedback.iter().all(|&fb| fb == Feedback::Green) {
            self.game_won = true;
            self.game_over = true;
            return;
        }

        // Check if out of guesses
        if self.remaining_guesses == 0 {
            self.game_over = true;
        }
    }

    fn draw(&self, f: &mut Frame) {
        // Horizontal split: 55% left (game), 45% right (analysis)
        let main_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
            .split(f.area());

        // Left column vertical layout (existing content)

        let left_constraints = vec![
            Constraint::Length(3), // Mode
            Constraint::Length(8), // Guesses
            Constraint::Min(5),    // Suggestions
            Constraint::Length(3), // Input
        ];

        let left_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(left_constraints)
            .split(main_layout[0]);

        // Right column analysis layout
        let right_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(8), // letter analysis
                Constraint::Length(9), // position analysis
                Constraint::Length(8), // constraint summary
                Constraint::Length(6), // solution pool
            ])
            .split(main_layout[1]);

        // Draw left column (existing logic)
        if self.mode == GameMode::Game {
            self.draw_game_status(f, left_layout[0]);
            self.draw_guesses(f, left_layout[1]);
            self.draw_suggestions(f, left_layout[2]);
            self.draw_input(f, left_layout[3]);
        } else {
            self.draw_mode_indicator(f, left_layout[0]);
            self.draw_guesses(f, left_layout[1]);
            self.draw_suggestions(f, left_layout[2]);
            self.draw_input(f, left_layout[3]);
        }

        // Draw right column (new analysis panels)
        self.draw_letter_analysis(f, right_layout[0]);
        self.draw_position_analysis(f, right_layout[1]);
        self.draw_constraint_summary(f, right_layout[2]);
        self.draw_solution_pool(f, right_layout[3]);
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
        let items: Vec<ListItem> = if self.suggestions.is_empty() {
            vec![ListItem::new("No suggestions yet")]
        } else {
            self.suggestions
                .iter()
                .take(10)
                .map(|(w, s)| ListItem::new(format!("{w} ({s})")))
                .collect()
        };

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

        let help_text = if self.mode == GameMode::Game {
            if self.game_over {
                "Enter = new game | Ctrl+S = solver | Ctrl+Q = quit"
            } else {
                "Enter = submit guess | Ctrl+S = solver | Ctrl+Q = quit"
            }
        } else {
            "Enter = submit | Ctrl+G = game | Ctrl+Z = undo | Ctrl+Q = quit"
        };

        f.render_widget(
            Paragraph::new(text).block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(border_color))
                    .title(format!("Input {} | {}", subtitle, help_text)),
            ),
            area,
        );
    }

    fn draw_mode_indicator(&self, f: &mut Frame, area: ratatui::layout::Rect) {
        let mode_text = format!(
            "Mode: {} | Press Ctrl+G for Game Mode",
            if self.mode == GameMode::Solver {
                "Solver"
            } else {
                "Game"
            }
        );

        f.render_widget(
            Paragraph::new(mode_text).block(Block::default().borders(Borders::ALL).title("Mode")),
            area,
        );
    }

    fn draw_game_status(&self, f: &mut Frame, area: ratatui::layout::Rect) {
        let status_text = if self.game_over {
            if self.game_won {
                format!(
                    "🎉 You Won! The word was: {}",
                    self.target_word
                        .as_ref()
                        .unwrap_or(&"?".to_string())
                        .to_uppercase()
                )
            } else {
                format!(
                    "💀 Game Over! The word was: {}",
                    self.target_word
                        .as_ref()
                        .unwrap_or(&"?".to_string())
                        .to_uppercase()
                )
            }
        } else {
            format!(
                "Guesses remaining: {} | Press Ctrl+S for Solver Mode",
                self.remaining_guesses
            )
        };

        let color = if self.game_won {
            Color::Green
        } else if self.game_over {
            Color::Red
        } else {
            Color::White
        };

        f.render_widget(
            Paragraph::new(status_text)
                .style(Style::default().fg(color))
                .block(Block::default().borders(Borders::ALL).title("Game Status")),
            area,
        );
    }

    fn draw_letter_analysis(&self, f: &mut Frame, area: ratatui::layout::Rect) {
        if let Some(analysis) = &self.letter_analysis {
            let mut lines = vec![
                Line::from("Letter Analysis"),
                Line::from(format!("(Remaining: {} words)", analysis.total_words)),
                Line::from(""), // spacer
            ];

            for c in ('A'..='Z').collect::<Vec<_>>() {
                if let Some(&count) = analysis.frequencies.get(&c) {
                    let bar_width = if analysis.max_frequency > 0 {
                        (count * 16 / analysis.max_frequency).max(1)
                    } else {
                        0
                    };
                    let bar = "█".repeat(bar_width);
                    let style = if count > 0 {
                        Style::default().fg(Color::White)
                    } else {
                        Style::default().fg(Color::DarkGray)
                    };

                    lines.push(Line::from(vec![
                        Span::raw(format!("{}: {:<3} ", c, count)),
                        Span::styled(bar, style),
                    ]));
                }
            }

            f.render_widget(
                Paragraph::new(lines)
                    .block(Block::default().borders(Borders::ALL).title("Analysis")),
                area,
            );
        }
    }

    fn draw_position_analysis(&self, f: &mut Frame, area: ratatui::layout::Rect) {
        if let Some(analysis) = &self.position_analysis {
            let mut lines = vec![Line::from("Position Analysis"), Line::from("")];

            for (pos, letters) in analysis.possible_letters.iter().enumerate() {
                let letters_with_freq: Vec<String> = letters
                    .iter()
                    .map(|c| {
                        if let Some(&count) = analysis.position_frequencies[pos].get(c) {
                            format!("{}({})", c, count)
                        } else {
                            c.to_string()
                        }
                    })
                    .collect();

                let letters_str = letters_with_freq.join(" ");

                lines.push(Line::from(format!("Pos {}: [{}]", pos + 1, letters_str)));
            }

            f.render_widget(
                Paragraph::new(lines)
                    .block(Block::default().borders(Borders::ALL).title("Positions")),
                area,
            );
        }
    }

    fn draw_constraint_summary(&self, f: &mut Frame, area: ratatui::layout::Rect) {
        if let Some(summary) = &self.constraint_summary {
            let mut lines = vec![Line::from("Active Constraints")];

            // Greens with guess source
            for (letter, pos, guess) in &summary.greens {
                lines.push(Line::from(vec![
                    Span::styled("✓ ", Style::default().fg(Color::Green)),
                    Span::raw(format!(
                        "{}({}) from '{}'",
                        letter,
                        pos + 1,
                        guess.to_uppercase()
                    )),
                ]));
            }

            // Yellows with guess source
            for (letter, positions, guess) in &summary.yellows {
                let pos_str: String = positions
                    .iter()
                    .map(|p| (p + 1).to_string())
                    .collect::<Vec<_>>()
                    .join(",");
                lines.push(Line::from(vec![
                    Span::styled("✓ ", Style::default().fg(Color::Yellow)),
                    Span::raw(format!(
                        "{}({}) from '{}'",
                        letter,
                        pos_str,
                        guess.to_uppercase()
                    )),
                ]));
            }

            // Grays
            if !summary.grays.is_empty() {
                let gray_str: String = summary
                    .grays
                    .iter()
                    .map(|c| c.to_string())
                    .collect::<Vec<_>>()
                    .join(" ");
                lines.push(Line::from(vec![
                    Span::styled("✗ ", Style::default().fg(Color::DarkGray)),
                    Span::raw(gray_str),
                ]));
            }

            f.render_widget(
                Paragraph::new(lines)
                    .block(Block::default().borders(Borders::ALL).title("Constraints")),
                area,
            );
        }
    }

    fn draw_solution_pool(&self, f: &mut Frame, area: ratatui::layout::Rect) {
        if let Some(stats) = &self.solution_pool_stats {
            let lines = vec![
                Line::from("Solution Pool"),
                Line::from(format!("Total: {} remaining", stats.total_remaining)),
                Line::from(format!(
                    "Filtered: {:.1}% eliminated",
                    stats.eliminated_percentage
                )),
                Line::from(format!("Entropy: {:.1} bits", stats.entropy)),
            ];

            f.render_widget(
                Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title("Pool")),
                area,
            );
        }
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
