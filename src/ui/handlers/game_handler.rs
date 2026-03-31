use crate::{
    solver::{Feedback, SolverState},
    wordlist::select_random_word,
};
use chrono::Utc;

use super::super::{app::App, history::GameOutcome, types::GameMode};

/// Helper struct for managing game-specific state transitions.
pub struct GameHandler<'a> {
    app: &'a mut App,
}

impl<'a> GameHandler<'a> {
    pub fn new(app: &'a mut App) -> Self {
        Self { app }
    }

    pub fn toggle_game_mode(&mut self) {
        if self.app.mode == GameMode::Solver {
            self.app.log("Starting new game");

            if self.app.solver_session_active {
                self.app.log("Solver session abandoned");
                self.app.solver_session_active = false;
                self.app.solver_session_start = None;
                self.app.solver_session_paused = false;
                self.app.solver_session_guesses.clear();
            }

            self.start_new_game();
        } else {
            self.app.log("Switching to solver mode");
            self.app.mode = GameMode::Solver;

            // Start a new solver session
            self.app.solver_session_active = true;
            self.app.solver_session_start = Some(Utc::now());
            self.app.solver_session_paused = false; // Ensure not paused
            self.app.log("Solver session started");

            SolverHandler::new(self.app).recompute();
            self.app.analysis_dirty = true;
        }
    }

    pub fn start_new_game(&mut self) {
        match select_random_word(&self.app.solution_words, self.app.solver.word_len()) {
            Ok(target) => {
                tracing::info!("New game started with target word: {}", target);
                self.app.mode = GameMode::Game;
                self.app.target_word = Some(target);
                self.app.remaining_guesses = 6;
                self.app.game_won = false;
                self.app.game_over = false;
                self.app.show_suggestions = false;
                self.app.show_analysis = false;
                self.app.solver = SolverState::new(self.app.solver.word_len());
                self.app.entropy_history.clear();
                self.app.input.clear();
                SolverHandler::new(self.app).recompute();
                self.app.analysis_dirty = true;
            }
            Err(_) => {
                self.app.log("Failed to start new game: no words available");
            }
        }
    }

    pub fn check_game_state(&mut self, feedback: &[Feedback]) {
        if feedback.iter().all(|&fb| fb == Feedback::Green) {
            self.app.log(format!(
                "Target word was {}",
                self.app.target_word.as_ref().unwrap()
            ));
            self.app.log("Game won!");
            self.app.game_won = true;
            self.app.game_over = true;
            let guesses = self.app.solver.guesses().len();
            self.save_completed_game(GameOutcome::Won { guesses });
            return;
        }

        if self.app.remaining_guesses == 0 {
            self.app.log("Game over: out of guesses");
            self.app.game_over = true;
            self.save_completed_game(GameOutcome::Lost);
        }
    }

    fn save_completed_game(&mut self, outcome: GameOutcome) {
        if let Some(ref target) = self.app.target_word.clone() {
            let guesses: Vec<_> = self.app.solver.guesses().to_vec();
            let timestamp = Utc::now();
            if let Err(e) = self.app.db.save_game(timestamp, target, &guesses, &outcome) {
                self.app.log(format!("Warning: failed to save game: {}", e));
            }
        }
    }
}

use super::SolverHandler;
