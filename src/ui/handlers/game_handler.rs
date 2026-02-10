//! Game mode state management.

use crate::{
    solver::{Feedback, SolverState},
    wordlist::select_random_word,
};

use super::super::{app::App, types::GameMode};

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
            self.start_new_game();
        } else {
            self.app.mode = GameMode::Solver;
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

    pub fn undo_guess(&mut self) {
        if !self.app.solver.guesses().is_empty() {
            self.app.solver.pop_guess();
            self.app.remaining_guesses += 1;
            self.app.game_won = false;
            self.app.game_over = false;
            SolverHandler::new(self.app).recompute();
            SolverHandler::new(self.app).rebuild_entropy_history();
            self.app.analysis_dirty = true;
        }
    }

    pub fn check_game_state(&mut self, feedback: &[Feedback]) {
        // Check if won (all green)
        if feedback.iter().all(|&fb| fb == Feedback::Green) {
            self.app.log(format!(
                "Target word was {}",
                self.app.target_word.as_ref().unwrap()
            ));
            self.app.log("Game won!");
            self.app.game_won = true;
            self.app.game_over = true;
            return;
        }

        // Check if out of guesses
        if self.app.remaining_guesses == 0 {
            self.app.log("Game over: out of guesses");
            self.app.game_over = true;
        }
    }
}

// Forward declaration - SolverHandler will be defined in solver_handler.rs
use super::SolverHandler;
