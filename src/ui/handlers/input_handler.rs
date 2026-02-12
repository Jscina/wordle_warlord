//! Input handling and validation.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::{
    analysis::compute_solution_pool_stats,
    db,
    scoring::{get_optimal_word, score_and_sort},
    solver::{generate_feedback, parse_pattern, Guess},
};

use super::super::{
    app::App,
    types::{GameMode, InputStatus, ParsedInput},
};
use super::{GameHandler, HistoryHandler, SolverHandler};

/// Helper struct for managing keyboard input and user interactions.
pub struct InputHandler<'a> {
    app: &'a mut App,
}

impl<'a> InputHandler<'a> {
    pub fn new(app: &'a mut App) -> Self {
        Self { app }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> bool {
        // Handle history mode navigation separately
        if self.app.mode == GameMode::History {
            return self.handle_history_key(key);
        }

        match (key.code, key.modifiers) {
            (KeyCode::Char('q' | 'Q'), KeyModifiers::CONTROL) => {
                self.app.log("Exit requested");
                return true;
            }

            (KeyCode::Char('g' | 'G'), KeyModifiers::CONTROL) => {
                self.app.log("Switching to game mode");
                GameHandler::new(self.app).toggle_game_mode();
            }

            (KeyCode::Char('s' | 'S'), KeyModifiers::CONTROL) => {
                if self.app.mode == GameMode::Game {
                    self.app.log("Switching to solver mode");
                    self.app.mode = GameMode::Solver;
                    SolverHandler::new(self.app).recompute();
                }
            }

            (KeyCode::Char('r' | 'R'), KeyModifiers::CONTROL) => {
                self.app.log("Switching to history mode");
                HistoryHandler::new(self.app).enter_history_mode();
            }

            (KeyCode::Char('h' | 'H'), KeyModifiers::CONTROL) => {
                if self.app.mode == GameMode::Game {
                    self.app.show_suggestions = !self.app.show_suggestions;
                    let status = if self.app.show_suggestions {
                        "shown"
                    } else {
                        "hidden"
                    };
                    self.app.log(format!("Suggestions {}", status));
                }
            }

            (KeyCode::Char('a' | 'A'), KeyModifiers::CONTROL) => {
                if self.app.mode == GameMode::Game {
                    self.app.show_analysis = !self.app.show_analysis;
                    let status = if self.app.show_analysis {
                        "shown"
                    } else {
                        "hidden"
                    };
                    self.app.log(format!("Analysis panels {}", status));
                }
            }

            (KeyCode::Char('z' | 'Z'), KeyModifiers::CONTROL) => {
                // Undo only works in Solver mode, not in Game mode
                if self.app.mode == GameMode::Solver {
                    self.app.log("Undo requested");
                    SolverHandler::new(self.app).undo_guess();
                }
            }

            (KeyCode::Enter, _) => self.submit_input(),
            (KeyCode::Backspace, _) => {
                self.app.input.pop();
            }
            (KeyCode::Char(c), _) => self.app.input.push(c),
            _ => {}
        }
        false
    }

    fn handle_history_key(&mut self, key: KeyEvent) -> bool {
        use super::super::history::HistoryViewMode;

        match key.code {
            KeyCode::Char('q' | 'Q') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.app.log("Exit requested");
                return true;
            }

            KeyCode::Char('r' | 'R') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.app.log("Returning to solver mode");
                HistoryHandler::new(self.app).exit_history_mode();
            }

            KeyCode::Tab => {
                HistoryHandler::new(self.app).cycle_view_mode();
            }

            KeyCode::PageDown => {
                if self.app.history_view_mode == HistoryViewMode::List {
                    HistoryHandler::new(self.app).next_page();
                }
            }

            KeyCode::PageUp => {
                if self.app.history_view_mode == HistoryViewMode::List {
                    HistoryHandler::new(self.app).prev_page();
                }
            }

            KeyCode::Esc => match self.app.history_view_mode {
                HistoryViewMode::Detail => {
                    HistoryHandler::new(self.app).return_to_list();
                }
                HistoryViewMode::List => {
                    HistoryHandler::new(self.app).return_to_stats();
                }
                _ => {}
            },

            KeyCode::Char(c @ '0'..='9') => {
                if self.app.history_view_mode == HistoryViewMode::List {
                    let digit = c.to_digit(10).unwrap() as usize;
                    // Map: 1-9 -> items 0-8, 0 -> item 9 (the 10th item)
                    let index = if digit == 0 { 9 } else { digit - 1 };
                    HistoryHandler::new(self.app).select_game_on_page(index);
                }
            }

            _ => {}
        }

        false
    }

    fn parse_input(&self) -> ParsedInput {
        if self.app.mode == GameMode::Game {
            let word = self.app.input.trim().to_lowercase();
            if word.len() != self.app.solver.word_len() {
                return ParsedInput::Invalid;
            }
            // In game mode, we don't parse pattern - it's generated
            return ParsedInput::Valid {
                word,
                feedback: Vec::new(),
            };
        }

        let parts: Vec<_> = self.app.input.split_whitespace().collect();
        if parts.len() != 2 {
            return ParsedInput::Incomplete;
        }

        let word = parts[0].to_lowercase();
        let pattern = parts[1];

        if word.len() != self.app.solver.word_len() || !self.app.allowed_lookup.contains(&word) {
            return ParsedInput::Invalid;
        }

        if pattern.len() != self.app.solver.word_len() {
            return ParsedInput::Invalid;
        }

        let feedback = match parse_pattern(pattern) {
            Ok(f) => f,
            Err(_) => return ParsedInput::Invalid,
        };

        ParsedInput::Valid { word, feedback }
    }

    pub fn input_status(&self) -> InputStatus {
        if self.app.mode == GameMode::Game {
            let guess = self.app.input.trim();

            if guess.is_empty() {
                return InputStatus::Incomplete;
            }

            if guess.len() != self.app.solver.word_len() {
                return InputStatus::Invalid("guess length mismatch");
            }

            let guess_lower = guess.to_lowercase();

            if !self.app.allowed_lookup.contains(&guess_lower) {
                return InputStatus::Invalid("word not in allowed list");
            }

            return InputStatus::Valid;
        }

        let parts: Vec<_> = self.app.input.split_whitespace().collect();

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

        if guess.len() != self.app.solver.word_len() {
            return InputStatus::Invalid("guess length mismatch");
        } else if !self.app.allowed_lookup.contains(&guess.to_lowercase()) {
            return InputStatus::Invalid("word not in allowed list");
        }

        if pattern.len() != self.app.solver.word_len() {
            return InputStatus::Invalid("pattern length mismatch");
        }

        if parse_pattern(pattern).is_err() {
            return InputStatus::Invalid("pattern must be G/Y/X");
        }

        InputStatus::Valid
    }

    fn submit_input(&mut self) {
        if self.app.mode == GameMode::Game && self.app.game_over {
            self.app.log("Starting new game");
            GameHandler::new(self.app).start_new_game();
            return;
        }

        if !matches!(self.input_status(), InputStatus::Valid) {
            self.app
                .log(format!("Input rejected: {:?}", self.app.input));
            return;
        }

        if self.app.mode == GameMode::Game {
            if let Some(ref target) = self.app.target_word {
                let word = self.app.input.trim().to_lowercase();

                if !self.app.allowed_lookup.contains(&word) {
                    self.app
                        .log(format!("Rejected guess not in allowed list: {}", word));
                    return;
                }

                self.app.log(format!("Game guess submitted: {}", &word));

                let feedback = generate_feedback(target, &word);

                self.app
                    .solver
                    .add_guess(Guess::new(word.clone(), feedback.clone()));

                self.app.remaining_guesses -= 1;

                // Save guess to database
                if let Some(game_id) = self.app.current_game_id {
                    let guess_number = (7 - self.app.remaining_guesses - 1) as i64;
                    let db_feedback: Vec<db::models::Feedback> = feedback
                        .iter()
                        .map(|f| db::models::Feedback::from_solver(f))
                        .collect();

                    let _ = self.app.run_db_operation(db::games::add_guess(
                        &self.app.db_pool,
                        game_id,
                        guess_number,
                        word,
                        db_feedback,
                    ));
                }

                GameHandler::new(self.app).check_game_state(&feedback);

                SolverHandler::new(self.app).recompute();
                self.app.input.clear();
            }
        } else if let ParsedInput::Valid { word, feedback } = self.parse_input() {
            if !self.app.allowed_lookup.contains(&word) {
                self.app
                    .log(format!("Rejected guess not in allowed list: {}", word));
                return;
            }

            // Calculate pool size and optimal word BEFORE applying the guess
            let remaining_before = self.app.solver.filter(&self.app.solution_words);
            let pool_size_before = remaining_before.len();

            // Get optimal word at this step (before applying the guess)
            let optimal = get_optimal_word(&remaining_before[..], &self.app.allowed_lookup);
            let (optimal_word, optimal_score) = optimal.unwrap_or((String::from("-----"), 0));

            // Get the score of the actual word chosen
            let actual_score = if pool_size_before > 0 {
                let scored = score_and_sort(&remaining_before[..], &self.app.allowed_lookup);
                scored
                    .iter()
                    .find(|(w, _)| w == &word)
                    .map(|(_, s)| *s)
                    .unwrap_or(0)
            } else {
                0
            };

            // Add the guess
            let guess = Guess::new(word.clone(), feedback.clone());
            self.app.solver.add_guess(guess);

            // Calculate pool size and entropy AFTER applying the guess
            let remaining_after = self.app.solver.filter(&self.app.solution_words);
            let pool_size_after = remaining_after.len();
            let stats = compute_solution_pool_stats(&self.app.solution_words, &remaining_after);
            let entropy = stats.entropy;

            // Calculate deviation: positive means actual is better, negative means optimal was better
            // Using score difference as a proxy for entropy difference
            let score_deviation = actual_score as f64 - optimal_score as f64;

            // Log with detailed solver session information
            if self.app.solver_session_active && !self.app.solver_session_paused {
                self.app.log(format!(
                    "Solver guess: {} (pool: {}→{}, entropy: {:.2}, optimal: {}, deviation: {:.2})",
                    &word,
                    pool_size_before,
                    pool_size_after,
                    entropy,
                    optimal_word,
                    score_deviation
                ));

                // Save guess to database
                if let Some(session_id) = self.app.current_session_id {
                    let guess_number = self.app.solver.guesses().len() as i64;
                    let _ = self.app.run_db_operation(db::solver::add_guess(
                        &self.app.db_pool,
                        session_id,
                        guess_number,
                        word.clone(),
                        pool_size_before as i64,
                        pool_size_after as i64,
                        entropy,
                        optimal_word.clone(),
                        optimal_score as f64,
                        score_deviation,
                    ));
                }
            } else {
                self.app
                    .log(format!("Solver guess submitted: {} {:?}", &word, feedback));
            }

            SolverHandler::new(self.app).recompute();
            self.app.input.clear();

            // Check for session completion: pool narrowed to 1 OR all green feedback
            let all_green = feedback
                .iter()
                .all(|f| *f == crate::solver::Feedback::Green);
            if self.app.solver_session_active
                && !self.app.solver_session_paused
                && (pool_size_after == 1 || all_green)
            {
                let guess_count = self.app.solver.guesses().len();
                self.app
                    .log(format!("Solver session completed: {} guesses", guess_count));

                // Update solver session outcome in database
                if let Some(session_id) = self.app.current_session_id {
                    let _ = self
                        .app
                        .run_db_operation(db::solver::update_session_outcome(
                            &self.app.db_pool,
                            session_id,
                            db::models::SolverOutcome::Completed,
                        ));
                }

                self.reset_solver_and_start_new_session();
            }
        }
    }

    /// Resets the solver state and starts a new session
    fn reset_solver_and_start_new_session(&mut self) {
        let word_len = self.app.solver.word_len();

        // Clear solver state
        self.app.solver = crate::solver::SolverState::new(word_len);
        self.app.entropy_history.clear();
        self.app.suggestions.clear();
        self.app.analysis_dirty = true;

        // Start new session
        let timestamp = chrono::Utc::now();
        self.app.solver_session_active = true;
        self.app.solver_session_paused = false;
        self.app.solver_session_start = Some(timestamp);
        self.app.log("Solver session started");

        // Create new session in database
        if let Ok(session_id) = self
            .app
            .run_db_operation(db::solver::create_session(&self.app.db_pool, timestamp))
        {
            self.app.current_session_id = Some(session_id);
        }
    }
}
