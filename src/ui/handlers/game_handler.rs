use crate::{
    db,
    solver::{Feedback, SolverState},
    wordlist::select_random_word,
};
use chrono::Utc;

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

            // End any active solver session
            if self.app.solver_session_active {
                self.app.log("Solver session abandoned");

                // Update solver session outcome in database
                if let Some(session_id) = self.app.current_session_id {
                    let _ = self
                        .app
                        .run_db_operation(db::solver::update_session_outcome(
                            &self.app.db_pool,
                            session_id,
                            db::models::SolverOutcome::Abandoned,
                        ));
                }

                self.app.solver_session_active = false;
                self.app.solver_session_start = None;
                self.app.solver_session_paused = false;
                self.app.current_session_id = None;
            }

            self.start_new_game();
        } else {
            self.app.log("Switching to solver mode");
            self.app.mode = GameMode::Solver;

            // Start a new solver session
            let timestamp = Utc::now();
            self.app.solver_session_active = true;
            self.app.solver_session_start = Some(timestamp);
            self.app.solver_session_paused = false;
            self.app.log("Solver session started");

            // Create solver session in database
            if let Ok(session_id) = self
                .app
                .run_db_operation(db::solver::create_session(&self.app.db_pool, timestamp))
            {
                self.app.current_session_id = Some(session_id);
            }

            SolverHandler::new(self.app).recompute();
            self.app.analysis_dirty = true;
        }
    }

    pub fn start_new_game(&mut self) {
        match select_random_word(&self.app.solution_words, self.app.solver.word_len()) {
            Ok(target) => {
                let timestamp = Utc::now();
                tracing::info!("New game started with target word: {}", target);

                // Create game in database
                if let Ok(game_id) = self.app.run_db_operation(db::games::create_game(
                    &self.app.db_pool,
                    timestamp,
                    target.clone(),
                )) {
                    self.app.current_game_id = Some(game_id);
                }

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
        // Check if won (all green)
        if feedback.iter().all(|&fb| fb == Feedback::Green) {
            self.app.log(format!(
                "Target word was {}",
                self.app.target_word.as_ref().unwrap()
            ));
            self.app.log("Game won!");
            self.app.game_won = true;
            self.app.game_over = true;

            // Update game outcome in database
            if let Some(game_id) = self.app.current_game_id {
                let _ = self.app.run_db_operation(db::games::update_game_outcome(
                    &self.app.db_pool,
                    game_id,
                    db::models::GameOutcome::Won,
                ));
            }

            return;
        }

        // Check if out of guesses
        if self.app.remaining_guesses == 0 {
            self.app.log("Game over: out of guesses");
            self.app.game_over = true;

            // Update game outcome in database
            if let Some(game_id) = self.app.current_game_id {
                let _ = self.app.run_db_operation(db::games::update_game_outcome(
                    &self.app.db_pool,
                    game_id,
                    db::models::GameOutcome::Lost,
                ));
            }
        }
    }
}

use super::SolverHandler;
