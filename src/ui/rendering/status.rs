use ratatui::{
    Frame,
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
};

use crate::ui::{app::App, types::GameMode};

impl App {
    pub(in crate::ui) fn draw_mode_indicator(&self, f: &mut Frame, area: ratatui::layout::Rect) {
        let mode_text = format!(
            "Mode: {} | Press Ctrl+G for Game Mode | Ctrl+R for History",
            if self.mode == GameMode::Solver {
                "Solver"
            } else {
                "Game"
            }
        );

        f.render_widget(
            Paragraph::new(mode_text).block(Block::default().borders(Borders::ALL).title("Mode")),
            area,
        );
    }

    pub(in crate::ui) fn draw_game_status(&self, f: &mut Frame, area: ratatui::layout::Rect) {
        let status_text = if self.game_over {
            if self.game_won {
                format!(
                    "🎉 You Won! The word was: {}",
                    self.target_word
                        .as_ref()
                        .unwrap_or(&"?".to_string())
                        .to_uppercase()
                )
            } else {
                format!(
                    "💀 Game Over! The word was: {}",
                    self.target_word
                        .as_ref()
                        .unwrap_or(&"?".to_string())
                        .to_uppercase()
                )
            }
        } else {
            format!(
                "Guesses remaining: {} | Ctrl+S: Solver | Ctrl+R: History",
                self.remaining_guesses
            )
        };

        let color = if self.game_won {
            Color::Green
        } else if self.game_over {
            Color::Red
        } else {
            Color::White
        };

        f.render_widget(
            Paragraph::new(status_text)
                .style(Style::default().fg(color))
                .block(Block::default().borders(Borders::ALL).title("Game Status")),
            area,
        );
    }
}
