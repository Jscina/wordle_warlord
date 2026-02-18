use super::super::{
    app::App,
    history::{HistoryData, HistoryViewMode},
    types::GameMode,
};

/// Helper struct for managing history mode state and operations.
pub struct HistoryHandler<'a> {
    app: &'a mut App,
}

impl<'a> HistoryHandler<'a> {
    pub fn new(app: &'a mut App) -> Self {
        Self { app }
    }

    /// Enter history mode by loading and parsing game history.
    pub fn enter_history_mode(&mut self) {
        self.app.mode = GameMode::History;
        self.app.history_view_mode = HistoryViewMode::Stats;
        self.app.history_page = 0;

        // Pause active solver session
        if self.app.solver_session_active && !self.app.solver_session_paused {
            self.app.solver_session_paused = true;
            self.app.log("Solver session paused");
        }

        // Load history if not already loaded
        if self.app.history_data.is_none() {
            self.load_history();
        }
    }

    /// Exit history mode and return to solver mode.
    pub fn exit_history_mode(&mut self) {
        self.app.mode = GameMode::Solver;

        // Resume solver session if it was paused
        if self.app.solver_session_active && self.app.solver_session_paused {
            self.app.solver_session_paused = false;
            self.app.log("Solver session resumed");
        }
    }

    /// Load and parse game history from the database.
    pub fn load_history(&mut self) {
        self.app.log("Loading game history...");

        // Use block_in_place to call async database operations
        let result = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                let games = crate::db::history::load_game_records(&self.app.db_pool).await?;
                let sessions = crate::db::history::load_solver_sessions(&self.app.db_pool).await?;
                Ok::<_, anyhow::Error>((games, sessions))
            })
        });

        match result {
            Ok((games, sessions)) => {
                let game_count = games.len();
                let session_count = sessions.len();

                self.app.history_data = Some(HistoryData::new(games, sessions));
                self.app.log(format!(
                    "Loaded {} game(s) and {} solver session(s) from history",
                    game_count, session_count
                ));
            }
            Err(e) => {
                self.app.log(format!("Failed to load history: {}", e));
                // Create empty history data so we can still show the UI
                self.app.history_data = Some(HistoryData::new(Vec::new(), Vec::new()));
            }
        }
    }

    /// Switch to the next view mode (Stats -> List -> Solver -> Stats).
    pub fn cycle_view_mode(&mut self) {
        self.app.history_view_mode = match self.app.history_view_mode {
            HistoryViewMode::Stats => HistoryViewMode::List,
            HistoryViewMode::List => {
                // If a game is selected, go to detail view
                if let Some(ref data) = self.app.history_data {
                    if data.selected_game().is_some() {
                        HistoryViewMode::Detail
                    } else {
                        HistoryViewMode::Solver
                    }
                } else {
                    HistoryViewMode::Solver
                }
            }
            HistoryViewMode::Detail => HistoryViewMode::Stats,
            HistoryViewMode::Solver => HistoryViewMode::Stats,
        };
    }

    /// Go to the next page in list view.
    pub fn next_page(&mut self) {
        if let Some(ref data) = self.app.history_data {
            let total_pages = data.total_pages();
            if total_pages > 0 && self.app.history_page < total_pages - 1 {
                self.app.history_page += 1;
            }
        }
    }

    /// Go to the previous page in list view.
    pub fn prev_page(&mut self) {
        if self.app.history_page > 0 {
            self.app.history_page -= 1;
        }
    }

    /// Select a game at the given index on the current page.
    pub fn select_game_on_page(&mut self, page_index: usize) {
        let global_index = self.app.history_page * 10 + page_index;
        if let Some(ref mut data) = self.app.history_data
            && global_index < data.games.len()
        {
            data.select_game(global_index);
            self.app.history_view_mode = HistoryViewMode::Detail;
        }
    }

    /// Return from detail view to list view.
    pub fn return_to_list(&mut self) {
        if let Some(ref mut data) = self.app.history_data {
            data.clear_selection();
        }
        self.app.history_view_mode = HistoryViewMode::List;
    }

    /// Return to stats view from any other view.
    pub fn return_to_stats(&mut self) {
        if let Some(ref mut data) = self.app.history_data {
            data.clear_selection();
        }
        self.app.history_view_mode = HistoryViewMode::Stats;
    }
}

#[cfg(test)]
mod tests {
    use super::super::super::{
        app::App,
        history::{GameOutcome, GameRecord, HistoryData, HistoryViewMode},
        types::{GameMode, LogBuffer},
    };
    use chrono::Utc;
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

    fn create_test_history_data() -> HistoryData {
        let games = vec![
            GameRecord {
                timestamp: Utc::now(),
                target_word: "stone".to_string(),
                guesses: vec![],
                outcome: GameOutcome::Won { guesses: 3 },
            },
            GameRecord {
                timestamp: Utc::now(),
                target_word: "raise".to_string(),
                guesses: vec![],
                outcome: GameOutcome::Lost,
            },
        ];
        HistoryData::new(games, Vec::new())
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_enter_history_mode() {
        let mut app = create_test_app().await;
        app.mode = GameMode::Solver;

        super::HistoryHandler::new(&mut app).enter_history_mode();

        assert_eq!(app.mode, GameMode::History);
        assert_eq!(app.history_view_mode, HistoryViewMode::Stats);
        assert_eq!(app.history_page, 0);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_exit_history_mode() {
        let mut app = create_test_app().await;
        app.mode = GameMode::History;

        super::HistoryHandler::new(&mut app).exit_history_mode();

        assert_eq!(app.mode, GameMode::Solver);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_cycle_view_mode_stats_to_list() {
        let mut app = create_test_app().await;
        app.history_view_mode = HistoryViewMode::Stats;
        app.history_data = Some(create_test_history_data());

        super::HistoryHandler::new(&mut app).cycle_view_mode();

        assert_eq!(app.history_view_mode, HistoryViewMode::List);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_cycle_view_mode_list_to_stats_no_selection() {
        let mut app = create_test_app().await;
        app.history_view_mode = HistoryViewMode::List;
        app.history_data = Some(create_test_history_data());

        super::HistoryHandler::new(&mut app).cycle_view_mode();

        // With the new Solver view, List cycles to Solver when no game is selected
        assert_eq!(app.history_view_mode, HistoryViewMode::Solver);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_cycle_view_mode_list_to_detail_with_selection() {
        let mut app = create_test_app().await;
        app.history_view_mode = HistoryViewMode::List;
        let mut data = create_test_history_data();
        data.select_game(0);
        app.history_data = Some(data);

        super::HistoryHandler::new(&mut app).cycle_view_mode();

        assert_eq!(app.history_view_mode, HistoryViewMode::Detail);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_cycle_view_mode_detail_to_stats() {
        let mut app = create_test_app().await;
        app.history_view_mode = HistoryViewMode::Detail;
        app.history_data = Some(create_test_history_data());

        super::HistoryHandler::new(&mut app).cycle_view_mode();

        assert_eq!(app.history_view_mode, HistoryViewMode::Stats);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_select_game_on_page() {
        let mut app = create_test_app().await;
        app.history_data = Some(create_test_history_data());
        app.history_page = 0;

        super::HistoryHandler::new(&mut app).select_game_on_page(0);

        assert_eq!(app.history_view_mode, HistoryViewMode::Detail);
        assert!(app.history_data.as_ref().unwrap().selected_game().is_some());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_select_game_invalid_index() {
        let mut app = create_test_app().await;
        app.history_data = Some(create_test_history_data());
        app.history_page = 0;

        // Try to select index that doesn't exist
        super::HistoryHandler::new(&mut app).select_game_on_page(99);

        // Should not crash, and no game should be selected
        assert!(app.history_data.as_ref().unwrap().selected_game().is_none());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_pagination() {
        let mut app = create_test_app().await;
        app.history_data = Some(create_test_history_data());
        app.history_page = 0;

        // Go to next page
        super::HistoryHandler::new(&mut app).next_page();

        // With only 2 games, should stay on page 0
        assert_eq!(app.history_page, 0);

        // Go back
        super::HistoryHandler::new(&mut app).prev_page();
        assert_eq!(app.history_page, 0);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_return_to_list() {
        let mut app = create_test_app().await;
        let mut data = create_test_history_data();
        data.select_game(0);
        app.history_data = Some(data);
        app.history_view_mode = HistoryViewMode::Detail;

        super::HistoryHandler::new(&mut app).return_to_list();

        assert_eq!(app.history_view_mode, HistoryViewMode::List);
        assert!(app.history_data.as_ref().unwrap().selected_game().is_none());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_return_to_stats() {
        let mut app = create_test_app().await;
        let mut data = create_test_history_data();
        data.select_game(0);
        app.history_data = Some(data);
        app.history_view_mode = HistoryViewMode::Detail;

        super::HistoryHandler::new(&mut app).return_to_stats();

        assert_eq!(app.history_view_mode, HistoryViewMode::Stats);
        assert!(app.history_data.as_ref().unwrap().selected_game().is_none());
    }
}
