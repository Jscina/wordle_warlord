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
use std::{
    collections::HashSet,
    fmt::Display,
    io::{Stdout, stdout},
    sync::{Arc, Mutex},
};
use tracing::info;

use crate::{
    analysis::{
        ConstraintSummary, LetterAnalysis, PositionAnalysis, SolutionPoolStats,
        compute_constraint_summary, compute_letter_analysis, compute_position_analysis,
        compute_solution_pool_stats,
    },
    scoring::score_and_sort,
    solver::{Feedback, Guess, SolverState, generate_feedback, parse_pattern},
    wordlist::{load_solutions, load_words, select_random_word},
};

const MAX_LOG_LINES: usize = 300;

#[derive(Clone)]
pub struct LogBuffer {
    inner: Arc<Mutex<Vec<String>>>,
}

impl LogBuffer {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn push(&self, msg: String) {
        let mut buf = self.inner.lock().unwrap();
        buf.push(msg);
        if buf.len() > MAX_LOG_LINES {
            buf.remove(0);
        }
    }

    pub fn lines(&self) -> Vec<String> {
        self.inner.lock().unwrap().clone()
    }
}

impl Default for LogBuffer {
    fn default() -> Self {
        Self::new()
    }
}

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
    solution_words: Vec<String>,
    allowed_lookup: HashSet<String>,
    solver: SolverState,
    input: String,
    suggestions: Vec<(String, usize)>,
    mode: GameMode,
    target_word: Option<String>,
    remaining_guesses: usize,
    game_won: bool,
    game_over: bool,
    letter_analysis: Option<LetterAnalysis>,
    position_analysis: Option<PositionAnalysis>,
    constraint_summary: Option<ConstraintSummary>,
    solution_pool_stats: Option<SolutionPoolStats>,
    entropy_history: Vec<f64>,
    analysis_dirty: bool,
    logs: LogBuffer,
}

impl App {
    pub fn new(
        words: Vec<String>,
        solution_words: Vec<String>,
        word_len: usize,
        logs: LogBuffer,
    ) -> Self {
        let allowed_lookup: HashSet<String> = words.iter().cloned().collect();

        Self {
            solution_words,
            allowed_lookup,
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
            entropy_history: Vec::new(),
            analysis_dirty: true,
            logs,
        }
    }

    pub fn run(&mut self, terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
        info!("UI started");
        self.logs.push("UI started".into());

        loop {
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

    fn log(&self, msg: impl Into<String> + Display) {
        tracing::info!("{}", &msg);
        self.logs.push(msg.into());
    }

    fn handle_key(&mut self, key: event::KeyEvent) -> bool {
        match (key.code, key.modifiers) {
            (KeyCode::Char('q'), KeyModifiers::CONTROL) => {
                self.log("Exit requested");
                return true;
            }

            (KeyCode::Char('g'), KeyModifiers::CONTROL) => {
                self.log("Switching to game mode");
                self.toggle_game_mode();
            }

            (KeyCode::Char('s'), KeyModifiers::CONTROL) => {
                if self.mode == GameMode::Game {
                    self.log("Switching to solver mode");
                    self.mode = GameMode::Solver;
                    self.recompute();
                }
            }

            (KeyCode::Char('z'), KeyModifiers::CONTROL) => {
                self.log("Undo requested");
                if self.mode == GameMode::Solver {
                    self.undo_last_guess();
                } else {
                    self.undo_game_guess();
                }
            }

            (KeyCode::Enter, _) => self.submit_input(),
            (KeyCode::Backspace, _) => {
                self.input.pop();
            }
            (KeyCode::Char(c), _) => self.input.push(c),
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
        } else if !self.allowed_lookup.contains(&word) {
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

            let guess_lower = guess.to_lowercase();

            if !self.allowed_lookup.contains(&guess_lower) {
                return InputStatus::Invalid("word not in allowed list");
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
        } else if !self.allowed_lookup.contains(&guess.to_lowercase()) {
            return InputStatus::Invalid("word not in allowed list");
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
            self.log("Starting new game");
            self.start_new_game();
            return;
        }

        if !matches!(self.input_status(), InputStatus::Valid) {
            self.log(format!("Input rejected: {:?}", self.input));
            return;
        }

        if self.mode == GameMode::Game {
            if let Some(ref target) = self.target_word {
                let word = self.input.trim().to_lowercase();

                if !self.allowed_lookup.contains(&word) {
                    self.log(format!("Rejected guess not in allowed list: {}", word));
                    return;
                }

                self.log(format!("Game guess submitted: {}", &word));

                let feedback = generate_feedback(target, &word);

                self.solver.add_guess(Guess::new(word, feedback.clone()));

                self.remaining_guesses -= 1;
                self.check_game_state(&feedback);

                self.recompute();
                self.input.clear();
            }
        } else if let ParsedInput::Valid { word, feedback } = self.parse_input() {
            if !self.allowed_lookup.contains(&word) {
                self.log(format!("Rejected guess not in allowed list: {}", word));
                return;
            }

            let guess = Guess::new(word.clone(), feedback.clone());
            self.log(format!("Solver guess submitted: {} {:?}", &word, feedback));
            self.solver.add_guess(guess);

            self.recompute();
            self.input.clear();
        }
    }

    fn recompute(&mut self) {
        let remaining = self.solver.filter(&self.solution_words);

        if self.solver.guesses().is_empty() {
            self.suggestions.clear();
        } else {
            self.suggestions = score_and_sort(&remaining, &self.allowed_lookup);
        }

        self.analysis_dirty = true;
    }

    fn recompute_analysis(&mut self) {
        if !self.analysis_dirty {
            return;
        }

        let remaining = self.solver.filter(&self.solution_words);

        self.letter_analysis = Some(compute_letter_analysis(&remaining));
        tracing::info!("LetterAnalysis: {:?}", self.letter_analysis);
        self.position_analysis = Some(compute_position_analysis(&remaining, &self.solver));
        tracing::info!("PositionAnalysis: {:?}", self.position_analysis);
        self.constraint_summary = Some(compute_constraint_summary(&self.solver));
        tracing::info!("ConstraintSummary: {:?}", self.constraint_summary);
        self.solution_pool_stats = Some(compute_solution_pool_stats(
            &self.solution_words,
            &remaining,
        ));

        tracing::info!("SolutionPoolStats: {:?}", self.solution_pool_stats);
        if let Some(stats) = &self.solution_pool_stats {
            // Only push if not rebuilding (i.e., during normal guess submission)
            if self.entropy_history.len() < self.solver.guesses().len() {
                self.entropy_history.push(stats.entropy);
            }
        }

        self.analysis_dirty = false;
    }

    fn undo_last_guess(&mut self) {
        if !self.solver.guesses().is_empty() {
            self.solver.pop_guess();
            self.recompute();
            self.rebuild_entropy_history_from_guesses();
            self.analysis_dirty = true;
        }
    }

    fn rebuild_entropy_history_from_guesses(&mut self) {
        self.entropy_history.clear();
        let guesses = self.solver.guesses();
        let mut temp_solver = SolverState::new(self.solver.word_len());
        for guess in guesses {
            temp_solver.add_guess(guess.clone());
            let remaining = temp_solver.filter(&self.solution_words);
            let stats = compute_solution_pool_stats(&self.solution_words, &remaining);
            self.entropy_history.push(stats.entropy);
        }
    }

    fn toggle_game_mode(&mut self) {
        if self.mode == GameMode::Solver {
            self.log("Starting new game");
            self.start_new_game();
        } else {
            self.mode = GameMode::Solver;
            self.recompute();
            self.analysis_dirty = true;
        }
    }

    fn start_new_game(&mut self) {
        match select_random_word(&self.solution_words, self.solver.word_len()) {
            Ok(target) => {
                tracing::info!("New game started with target word: {}", target);
                self.mode = GameMode::Game;
                self.target_word = Some(target);
                self.remaining_guesses = 6;
                self.game_won = false;
                self.game_over = false;
                self.solver = SolverState::new(self.solver.word_len());
                self.entropy_history.clear();
                self.input.clear();
                self.recompute();
                self.analysis_dirty = true;
            }
            Err(_) => {
                self.log("Failed to start new game: no words available");
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
            self.rebuild_entropy_history_from_guesses();
            self.analysis_dirty = true;
        }
    }

    fn check_game_state(&mut self, feedback: &[Feedback]) {
        // Check if won (all green)
        if feedback.iter().all(|&fb| fb == Feedback::Green) {
            self.log(format!(
                "Target word was {}",
                self.target_word.as_ref().unwrap()
            ));
            self.log("Game won!");
            self.game_won = true;
            self.game_over = true;
            return;
        }

        // Check if out of guesses
        if self.remaining_guesses == 0 {
            self.log("Game over: out of guesses");
            self.game_over = true;
        }
    }

    fn draw(&self, f: &mut Frame) {
        let main_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
            .split(f.area());

        let left_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(8),
                Constraint::Min(5),
                Constraint::Length(3),
            ])
            .split(main_layout[0]);

        let right_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(8),
                Constraint::Length(8),
                Constraint::Length(8),
                Constraint::Length(12),
                Constraint::Min(6), // logs panel
            ])
            .split(main_layout[1]);

        if self.mode == GameMode::Game {
            self.draw_game_status(f, left_layout[0]);
        } else {
            self.draw_mode_indicator(f, left_layout[0]);
        }

        self.draw_guesses(f, left_layout[1]);
        self.draw_suggestions(f, left_layout[2]);
        self.draw_input(f, left_layout[3]);

        self.draw_letter_analysis(f, right_layout[0]);
        self.draw_position_analysis(f, right_layout[1]);
        self.draw_constraint_summary(f, right_layout[2]);
        self.draw_solution_pool(f, right_layout[3]);
        self.draw_logs(f, right_layout[4]);
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
            let mut freq: Vec<(char, usize)> =
                analysis.frequencies.iter().map(|(c, v)| (*c, *v)).collect();

            // Sort by frequency descending
            freq.sort_by(|a, b| b.1.cmp(&a.1));

            let max_bar = area.width.saturating_sub(8) as usize;

            let mut lines = vec![
                Line::from(format!("Remaining: {} words", analysis.total_words)),
                Line::from(""),
            ];

            for (c, count) in freq.into_iter().take(10) {
                let width = if analysis.max_frequency > 0 {
                    (count * max_bar / analysis.max_frequency).max(1)
                } else {
                    0
                };

                let bar = "█".repeat(width);

                lines.push(Line::from(vec![
                    Span::raw(format!("{} {:>4} ", c, count)),
                    Span::styled(bar, Style::default().fg(Color::Cyan)),
                ]));
            }

            f.render_widget(
                Paragraph::new(lines)
                    .block(Block::default().borders(Borders::ALL).title("Letters")),
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
            let mut lines = vec![
                Line::from("Solution Pool"),
                Line::from(format!("Total: {} remaining", stats.total_remaining)),
                Line::from(format!(
                    "Filtered: {:.1}% eliminated",
                    stats.eliminated_percentage
                )),
                Line::from(format!("Entropy: {:.2} bits", stats.entropy)),
                Line::from(""),
            ];

            if !self.entropy_history.is_empty() {
                let graph_height = area.height.saturating_sub(lines.len() as u16 + 2) as usize;
                let graph_width = area.width.saturating_sub(2) as usize;

                if graph_height > 0 && graph_width > 0 {
                    let start = self.entropy_history.len().saturating_sub(graph_width);
                    let slice = &self.entropy_history[start..];

                    let max_entropy = slice.iter().cloned().fold(0.0_f64, f64::max);

                    let mut grid = vec![vec![' '; graph_width]; graph_height];

                    for (i, value) in slice.iter().enumerate() {
                        let x = i;

                        let normalized = if max_entropy > 0.0 {
                            ((value + 1.0).ln() / (max_entropy + 1.0).ln()).clamp(0.0, 1.0)
                        } else {
                            0.0
                        };

                        let y =
                            graph_height - 1 - (normalized * (graph_height as f64 - 1.0)) as usize;

                        grid[y][x] = '●';
                    }

                    for row in grid {
                        lines.push(Line::from(row.into_iter().collect::<String>()));
                    }
                }
            }

            f.render_widget(
                Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title("Pool")),
                area,
            );
        }
    }

    fn draw_logs(&self, f: &mut Frame, area: ratatui::layout::Rect) {
        let logs = self.logs.lines();

        let height = area.height as usize;
        let start = logs.len().saturating_sub(height);

        let lines: Vec<Line> = logs[start..]
            .iter()
            .map(|l| Line::from(l.clone()))
            .collect();

        f.render_widget(
            Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title("Logs")),
            area,
        );
    }
}

pub fn run_ui() -> Result<()> {
    let words = load_words()?;
    let solution_words = load_solutions()?;
    let logs = LogBuffer::new();

    let mut app = App::new(words, solution_words, 5, logs.clone());

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
