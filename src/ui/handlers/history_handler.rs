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

        self.load_history();
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

    pub fn load_history(&mut self) {
        self.app.log("Loading game history...");

        let games = self.app.db.load_games().unwrap_or_else(|e| {
            self.app.log(format!("Failed to load games: {}", e));
            Vec::new()
        });

        let sessions = self.app.db.load_solver_sessions().unwrap_or_else(|e| {
            self.app
                .log(format!("Failed to load solver sessions: {}", e));
            Vec::new()
        });

        let game_count = games.len();
        let session_count = sessions.len();
        self.app.history_data = Some(HistoryData::new(games, sessions));
        self.app.log(format!(
            "Loaded {} game(s) and {} solver session(s) from history",
            game_count, session_count
        ));
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
