use std::{collections::HashSet, fmt::Display, io::Stdout};

use anyhow::Result;
use chrono::{DateTime, Utc};
use crossterm::event::{self, Event};
use ratatui::{backend::CrosstermBackend, Terminal};
use sqlx::SqlitePool;
use tracing::info;

use crate::{
    analysis::{ConstraintSummary, LetterAnalysis, PositionAnalysis, SolutionPoolStats},
    solver::SolverState,
};

use super::{
    history::{HistoryData, HistoryViewMode},
    types::{GameMode, LogBuffer},
};

/// Main application state container.
pub struct App {
    pub(in crate::ui) solution_words: Vec<String>,
    pub(in crate::ui) allowed_lookup: HashSet<String>,
    pub(in crate::ui) solver: SolverState,
    pub(in crate::ui) input: String,
    pub(in crate::ui) suggestions: Vec<(String, usize)>,
    pub(in crate::ui) mode: GameMode,
    pub(in crate::ui) target_word: Option<String>,
    pub(in crate::ui) remaining_guesses: usize,
    pub(in crate::ui) game_won: bool,
    pub(in crate::ui) game_over: bool,
    pub(in crate::ui) show_suggestions: bool,
    pub(in crate::ui) show_analysis: bool,
    pub(in crate::ui) letter_analysis: Option<LetterAnalysis>,
    pub(in crate::ui) position_analysis: Option<PositionAnalysis>,
    pub(in crate::ui) constraint_summary: Option<ConstraintSummary>,
    pub(in crate::ui) solution_pool_stats: Option<SolutionPoolStats>,
    pub(in crate::ui) entropy_history: Vec<f64>,
    pub(in crate::ui) analysis_dirty: bool,
    pub(in crate::ui) logs: LogBuffer,
    pub(in crate::ui) history_data: Option<HistoryData>,
    pub(in crate::ui) history_view_mode: HistoryViewMode,
    pub(in crate::ui) history_page: usize,
    pub(in crate::ui) solver_session_active: bool,
    pub(in crate::ui) solver_session_start: Option<DateTime<Utc>>,
    pub(in crate::ui) solver_session_paused: bool,
    pub(in crate::ui) db_pool: SqlitePool,
    pub(in crate::ui) current_game_id: Option<i64>,
    pub(in crate::ui) current_session_id: Option<i64>,
}

impl App {
    pub fn new(
        words: Vec<String>,
        solution_words: Vec<String>,
        word_len: usize,
        logs: LogBuffer,
        db_pool: SqlitePool,
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
            show_suggestions: true,
            show_analysis: true,
            letter_analysis: None,
            position_analysis: None,
            constraint_summary: None,
            solution_pool_stats: None,
            entropy_history: Vec::new(),
            analysis_dirty: true,
            logs,
            history_data: None,
            history_view_mode: HistoryViewMode::Stats,
            history_page: 0,
            solver_session_active: true, // Start with session active since we're in Solver mode
            solver_session_start: Some(Utc::now()),
            solver_session_paused: false,
            db_pool,
            current_game_id: None,
            current_session_id: None,
        }
    }

    pub fn run(&mut self, terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
        info!("UI started");
        self.log("UI started");

        // Log solver session start (app starts in Solver mode)
        if self.solver_session_active {
            self.log("Solver session started");
        }

        loop {
            // Recompute analysis if needed
            super::handlers::SolverHandler::new(self).recompute_analysis();

            terminal.draw(|f| self.draw(f))?;

            let event = event::read()?;
            if let Event::Key(key) = event {
                // Use InputHandler to process keyboard input
                if super::handlers::InputHandler::new(self).handle_key(key) {
                    return Ok(());
                }
            }
        }
    }

    pub(in crate::ui) fn log(&self, msg: impl Into<String> + Display) {
        tracing::info!("{}", &msg);
        self.logs.push(msg.into());
    }

    /// Execute an async database operation from sync context
    pub(in crate::ui) fn run_db_operation<F, T>(&self, future: F) -> Result<T>
    where
        F: std::future::Future<Output = Result<T>>,
    {
        tokio::task::block_in_place(|| tokio::runtime::Handle::current().block_on(future))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solver::{Feedback, Guess};
    use sqlx::sqlite::SqlitePoolOptions;

    /// Helper function to create a test database pool (in-memory SQLite).
    async fn create_test_db_pool() -> SqlitePool {
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
    async fn test_app_initialization() {
        let app = create_test_app().await;

        assert_eq!(app.mode, GameMode::Solver);
        assert!(app.input.is_empty());
        assert_eq!(app.remaining_guesses, 6);
        assert!(!app.game_won);
        assert!(!app.game_over);
        assert!(app.show_suggestions);
        assert!(app.show_analysis);
        assert!(app.target_word.is_none());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_log_buffer() {
        let logs = LogBuffer::new();

        logs.push("Test message 1".to_string());
        logs.push("Test message 2".to_string());

        let lines = logs.lines();
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0], "Test message 1");
        assert_eq!(lines[1], "Test message 2");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_log_buffer_max_capacity() {
        let logs = LogBuffer::new();

        // Push more than MAX_LOG_LINES
        for i in 0..350 {
            logs.push(format!("Message {}", i));
        }

        let lines = logs.lines();
        assert!(lines.len() <= super::super::types::MAX_LOG_LINES);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_solver_to_game_transition() {
        let mut app = create_test_app().await;
        app.mode = GameMode::Solver;
        app.show_analysis = true;

        super::super::handlers::GameHandler::new(&mut app).toggle_game_mode();

        assert_eq!(app.mode, GameMode::Game);
        assert!(!app.show_analysis); // Should be hidden in game mode
        assert!(!app.show_suggestions); // Should be hidden in game mode
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_game_to_solver_transition() {
        let mut app = create_test_app().await;
        super::super::handlers::GameHandler::new(&mut app).start_new_game();
        app.show_analysis = false;

        super::super::handlers::GameHandler::new(&mut app).toggle_game_mode();

        assert_eq!(app.mode, GameMode::Solver);
        // Analysis visibility should remain as set (solver mode respects toggle)
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_solver_to_history_transition() {
        let mut app = create_test_app().await;
        app.mode = GameMode::Solver;

        super::super::handlers::HistoryHandler::new(&mut app).enter_history_mode();

        assert_eq!(app.mode, GameMode::History);
        assert_eq!(app.history_view_mode, HistoryViewMode::Stats);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_history_to_solver_transition() {
        let mut app = create_test_app().await;
        app.mode = GameMode::History;

        super::super::handlers::HistoryHandler::new(&mut app).exit_history_mode();

        assert_eq!(app.mode, GameMode::Solver);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_game_state_persists_when_switching_modes() {
        let mut app = create_test_app().await;

        // Add a guess in solver mode
        let guess = Guess::new(
            "raise".to_string(),
            vec![
                Feedback::Gray,
                Feedback::Yellow,
                Feedback::Gray,
                Feedback::Gray,
                Feedback::Green,
            ],
        );
        app.solver.add_guess(guess);
        let guess_count = app.solver.guesses().len();

        // Switch to history and back
        super::super::handlers::HistoryHandler::new(&mut app).enter_history_mode();
        super::super::handlers::HistoryHandler::new(&mut app).exit_history_mode();

        // Solver state should be preserved
        assert_eq!(app.solver.guesses().len(), guess_count);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_analysis_shown_by_default_in_solver() {
        let app = create_test_app().await;

        assert_eq!(app.mode, GameMode::Solver);
        assert!(app.show_analysis);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_analysis_hidden_by_default_in_game() {
        let mut app = create_test_app().await;

        super::super::handlers::GameHandler::new(&mut app).start_new_game();

        assert_eq!(app.mode, GameMode::Game);
        assert!(!app.show_analysis);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_analysis_toggle_in_game_mode() {
        let mut app = create_test_app().await;
        super::super::handlers::GameHandler::new(&mut app).start_new_game();

        assert!(!app.show_analysis);

        // Toggle on
        app.show_analysis = !app.show_analysis;
        assert!(app.show_analysis);

        // Toggle off
        app.show_analysis = !app.show_analysis;
        assert!(!app.show_analysis);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_suggestions_toggle_in_game_mode() {
        let mut app = create_test_app().await;
        super::super::handlers::GameHandler::new(&mut app).start_new_game();

        assert!(!app.show_suggestions);

        // Toggle on
        app.show_suggestions = !app.show_suggestions;
        assert!(app.show_suggestions);

        // Toggle off
        app.show_suggestions = !app.show_suggestions;
        assert!(!app.show_suggestions);
    }
}
