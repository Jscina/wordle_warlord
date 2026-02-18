use crate::{
    analysis::{
        compute_constraint_summary, compute_letter_analysis, compute_position_analysis,
        compute_solution_pool_stats,
    },
    db,
    scoring::score_and_sort,
    solver::SolverState,
};

use super::super::app::App;

/// Helper struct for managing solver-specific state and analysis.
pub struct SolverHandler<'a> {
    app: &'a mut App,
}

impl<'a> SolverHandler<'a> {
    pub fn new(app: &'a mut App) -> Self {
        Self { app }
    }

    pub fn recompute(&mut self) {
        let remaining = self.app.solver.filter(&self.app.solution_words);

        if self.app.solver.guesses().is_empty() {
            self.app.suggestions.clear();
        } else {
            self.app.suggestions = score_and_sort(&remaining, &self.app.allowed_lookup);
        }

        self.app.analysis_dirty = true;
    }

    pub fn recompute_analysis(&mut self) {
        if !self.app.analysis_dirty {
            return;
        }

        let remaining = self.app.solver.filter(&self.app.solution_words);

        self.app.letter_analysis = Some(compute_letter_analysis(&remaining));
        tracing::info!("LetterAnalysis: {:?}", self.app.letter_analysis);
        self.app.position_analysis = Some(compute_position_analysis(&remaining, &self.app.solver));
        tracing::info!("PositionAnalysis: {:?}", self.app.position_analysis);
        self.app.constraint_summary = Some(compute_constraint_summary(&self.app.solver));
        tracing::info!("ConstraintSummary: {:?}", self.app.constraint_summary);
        self.app.solution_pool_stats = Some(compute_solution_pool_stats(
            &self.app.solution_words,
            &remaining,
        ));

        tracing::info!("SolutionPoolStats: {:?}", self.app.solution_pool_stats);
        if let Some(stats) = &self.app.solution_pool_stats {
            // Only push if not rebuilding (i.e., during normal guess submission)
            if self.app.entropy_history.len() < self.app.solver.guesses().len() {
                self.app.entropy_history.push(stats.entropy);
            }
        }

        self.app.analysis_dirty = false;
    }

    pub fn undo_guess(&mut self) {
        if !self.app.solver.guesses().is_empty() {
            if self.app.solver_session_active && !self.app.solver_session_paused {
                // Log undo in solver session
                let last_guess = self.app.solver.guesses().last().unwrap();
                tracing::info!("Solver undo: removed guess {}", last_guess.word);

                // Remove last guess from database
                if let Some(session_id) = self.app.current_session_id {
                    let _ = self.app.run_db_operation(db::solver::remove_last_guess(
                        &self.app.db_pool,
                        session_id,
                    ));
                }
            }
            self.app.solver.pop_guess();
            self.recompute();
            self.rebuild_entropy_history();
            self.app.analysis_dirty = true;
        }
    }

    pub fn rebuild_entropy_history(&mut self) {
        self.app.entropy_history.clear();
        let guesses = self.app.solver.guesses();
        let mut temp_solver = SolverState::new(self.app.solver.word_len());
        for guess in guesses {
            temp_solver.add_guess(guess.clone());
            let remaining = temp_solver.filter(&self.app.solution_words);
            let stats = compute_solution_pool_stats(&self.app.solution_words, &remaining);
            self.app.entropy_history.push(stats.entropy);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::super::{app::App, types::LogBuffer};
    use crate::solver::{Feedback, Guess};
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
    async fn test_undo_guess() {
        let mut app = create_test_app().await;

        // Add a guess
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

        assert_eq!(app.solver.guesses().len(), 1);

        // Undo the guess
        super::SolverHandler::new(&mut app).undo_guess();

        assert_eq!(app.solver.guesses().len(), 0);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_undo_guess_empty() {
        let mut app = create_test_app().await;

        assert_eq!(app.solver.guesses().len(), 0);

        // Undo with no guesses should not crash
        super::SolverHandler::new(&mut app).undo_guess();

        assert_eq!(app.solver.guesses().len(), 0);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_recompute_updates_suggestions() {
        let mut app = create_test_app().await;

        // Add a guess first - recompute only generates suggestions when guesses exist
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

        // Initially no suggestions computed
        app.suggestions.clear();

        // Recompute should populate suggestions
        super::SolverHandler::new(&mut app).recompute();

        assert!(!app.suggestions.is_empty());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_recompute_with_guess_narrows_suggestions() {
        let mut app = create_test_app().await;

        // Get initial suggestion count
        super::SolverHandler::new(&mut app).recompute();
        let initial_count = app.suggestions.len();

        // Add a guess that filters words
        let guess = Guess::new(
            "raise".to_string(),
            vec![
                Feedback::Gray,
                Feedback::Gray,
                Feedback::Gray,
                Feedback::Gray,
                Feedback::Green,
            ],
        );
        app.solver.add_guess(guess);

        // Recompute with constraint
        super::SolverHandler::new(&mut app).recompute();
        let filtered_count = app.suggestions.len();

        // Should have fewer suggestions after constraint
        assert!(filtered_count <= initial_count);
    }
}
