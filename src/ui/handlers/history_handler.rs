//! History mode state management.

use super::super::{
    app::App,
    history::{parse_game_history, HistoryData, HistoryViewMode},
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

        // Load history if not already loaded
        if self.app.history_data.is_none() {
            self.load_history();
        }
    }

    /// Exit history mode and return to solver mode.
    pub fn exit_history_mode(&mut self) {
        self.app.mode = GameMode::Solver;
    }

    /// Load and parse game history from log files.
    pub fn load_history(&mut self) {
        self.app.log("Loading game history...");

        match parse_game_history("logs") {
            Ok(games) => {
                let game_count = games.len();
                self.app.history_data = Some(HistoryData::new(games));
                self.app
                    .log(format!("Loaded {} game(s) from history", game_count));
            }
            Err(e) => {
                self.app.log(format!("Failed to load history: {}", e));
                // Create empty history data so we can still show the UI
                self.app.history_data = Some(HistoryData::new(Vec::new()));
            }
        }
    }

    /// Reload history from log files (in case new games were played).
    pub fn reload_history(&mut self) {
        self.app.history_data = None;
        self.load_history();
    }

    /// Switch to the next view mode (Stats -> List -> Detail -> Stats).
    pub fn cycle_view_mode(&mut self) {
        self.app.history_view_mode = match self.app.history_view_mode {
            HistoryViewMode::Stats => HistoryViewMode::List,
            HistoryViewMode::List => {
                // If a game is selected, go to detail view
                if let Some(ref data) = self.app.history_data {
                    if data.selected_game().is_some() {
                        HistoryViewMode::Detail
                    } else {
                        HistoryViewMode::Stats
                    }
                } else {
                    HistoryViewMode::Stats
                }
            }
            HistoryViewMode::Detail => HistoryViewMode::Stats,
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
        if let Some(ref mut data) = self.app.history_data {
            if global_index < data.games.len() {
                data.select_game(global_index);
                self.app.history_view_mode = HistoryViewMode::Detail;
            }
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
