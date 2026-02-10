//! Main rendering orchestration and layout management.

pub mod analysis;
mod guesses;
mod input_field;
mod logs;
mod status;
mod suggestions;

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
};

use crate::ui::{app::App, types::GameMode};

impl App {
    pub(in crate::ui) fn draw(&self, f: &mut Frame) {
        let main_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
            .split(f.area());

        // Dynamically adjust left layout based on whether suggestions should be shown
        let show_suggestions_panel = self.mode == GameMode::Solver || self.show_suggestions;

        let left_layout = if show_suggestions_panel {
            Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),
                    Constraint::Length(8),
                    Constraint::Min(5),
                    Constraint::Length(3),
                ])
                .split(main_layout[0])
        } else {
            Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),
                    Constraint::Length(8),
                    Constraint::Length(3),
                ])
                .split(main_layout[0])
        };

        let right_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(8),
                Constraint::Length(9),
                Constraint::Length(8),
                Constraint::Length(12),
                Constraint::Min(6), // logs panel
            ])
            .split(main_layout[1]);

        if self.mode == GameMode::Game {
            self.draw_game_status(f, left_layout[0]);
        } else {
            self.draw_mode_indicator(f, left_layout[0]);
        }

        self.draw_guesses(f, left_layout[1]);

        if show_suggestions_panel {
            self.draw_suggestions(f, left_layout[2]);
            self.draw_input(f, left_layout[3]);
        } else {
            self.draw_input(f, left_layout[2]);
        }

        self.draw_letter_analysis(f, right_layout[0]);
        self.draw_position_analysis(f, right_layout[1]);
        self.draw_constraint_summary(f, right_layout[2]);
        self.draw_solution_pool(f, right_layout[3]);
        self.draw_logs(f, right_layout[4]);
    }
}
