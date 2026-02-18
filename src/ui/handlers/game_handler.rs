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

#[cfg(test)]
mod tests {
    use super::super::super::{app::App, types::{GameMode, LogBuffer}};
    use crate::solver::Feedback;
    use sqlx::sqlite::SqlitePoolOptions;

    /// Helper function to create a test database pool (in-memory SQLite).
    async fn create_test_db_pool() -> sqlx::Pool<sqlx::Sqlite> {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("Failed to create test database pool");

        // Run migrations
        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .expect("Failed to run migrations on test database");

        pool
    }

    /// Helper function to create a test app with a minimal word list.
    async fn create_test_app() -> App {
        let words = vec![
            "raise".to_string(),
            "stone".to_string(),
            "slate".to_string(),
            "crane".to_string(),
            "house".to_string(),
            "apple".to_string(),
            "world".to_string(),
            "magic".to_string(),
        ];
        let solution_words = words.clone();
        let logs = LogBuffer::new();
        let db_pool = create_test_db_pool().await;

        App::new(words, solution_words, 5, logs, db_pool)
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_start_new_game() {
        let mut app = create_test_app().await;

        super::GameHandler::new(&mut app).start_new_game();

        assert_eq!(app.mode, GameMode::Game);
        assert!(app.target_word.is_some());
        assert_eq!(app.remaining_guesses, 6);
        assert!(!app.game_won);
        assert!(!app.game_over);
        assert!(!app.show_suggestions);
        assert!(!app.show_analysis);
        assert!(app.input.is_empty());
        assert_eq!(app.solver.guesses().len(), 0);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_toggle_game_mode_from_solver() {
        let mut app = create_test_app().await;
        app.mode = GameMode::Solver;

        super::GameHandler::new(&mut app).toggle_game_mode();

        assert_eq!(app.mode, GameMode::Game);
        assert!(app.target_word.is_some());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_toggle_game_mode_from_game() {
        let mut app = create_test_app().await;
        super::GameHandler::new(&mut app).start_new_game();

        super::GameHandler::new(&mut app).toggle_game_mode();

        assert_eq!(app.mode, GameMode::Solver);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_check_game_state_won() {
        let mut app = create_test_app().await;
        app.mode = GameMode::Game;
        app.target_word = Some("stone".to_string());

        let all_green = vec![
            Feedback::Green,
            Feedback::Green,
            Feedback::Green,
            Feedback::Green,
            Feedback::Green,
        ];

        super::GameHandler::new(&mut app).check_game_state(&all_green);

        assert!(app.game_won);
        assert!(app.game_over);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_check_game_state_not_won() {
        let mut app = create_test_app().await;
        app.mode = GameMode::Game;
        app.target_word = Some("stone".to_string());
        app.remaining_guesses = 3;

        let mixed = vec![
            Feedback::Green,
            Feedback::Yellow,
            Feedback::Gray,
            Feedback::Green,
            Feedback::Gray,
        ];

        super::GameHandler::new(&mut app).check_game_state(&mixed);

        assert!(!app.game_won);
        assert!(!app.game_over);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_check_game_state_out_of_guesses() {
        let mut app = create_test_app().await;
        app.mode = GameMode::Game;
        app.target_word = Some("stone".to_string());
        app.remaining_guesses = 0;

        let mixed = vec![
            Feedback::Green,
            Feedback::Yellow,
            Feedback::Gray,
            Feedback::Green,
            Feedback::Gray,
        ];

        super::GameHandler::new(&mut app).check_game_state(&mixed);

        assert!(!app.game_won);
        assert!(app.game_over);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_game_state_resets_on_new_game() {
        let mut app = create_test_app().await;

        // Start first game
        super::GameHandler::new(&mut app).start_new_game();
        app.remaining_guesses = 2;
        app.game_over = true;
        app.game_won = true;

        // Start new game
        super::GameHandler::new(&mut app).start_new_game();

        assert_eq!(app.remaining_guesses, 6);
        assert!(!app.game_over);
        assert!(!app.game_won);
        assert_eq!(app.solver.guesses().len(), 0);
    }
}
