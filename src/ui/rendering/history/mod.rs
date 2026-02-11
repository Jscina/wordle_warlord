//! History mode rendering coordinator.

mod detail_view;
mod list_view;
mod solver_view;
mod stats_view;

use ratatui::{layout::Rect, Frame};

use crate::ui::{history::HistoryViewMode, App};

impl App {
    pub(in crate::ui) fn draw_history_mode(&self, f: &mut Frame, area: Rect) {
        match self.history_view_mode {
            HistoryViewMode::Stats => self.draw_stats_view(f, area),
            HistoryViewMode::List => self.draw_list_view(f, area),
            HistoryViewMode::Detail => self.draw_detail_view(f, area),
            HistoryViewMode::Solver => self.draw_solver_view(f, area),
        }
    }
}
