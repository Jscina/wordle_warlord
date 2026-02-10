//! Input field rendering with validation status.

use ratatui::{
    Frame,
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
};

use crate::ui::{
    app::App,
    types::{GameMode, InputStatus},
};

impl App {
    pub(in crate::ui) fn draw_input(&self, f: &mut Frame, area: ratatui::layout::Rect) {
        let status = self.input_status_immutable();

        let (border_color, subtitle) = match status {
            InputStatus::Incomplete => (Color::Gray, ""),
            InputStatus::Valid => (Color::Green, ""),
            InputStatus::Invalid(msg) => (Color::Red, msg),
        };

        let text = format!("{}▌", self.input);

        let help_text = if self.mode == GameMode::Game {
            if self.game_over {
                "Enter = new game | Ctrl+S = solver | Ctrl+Q = quit"
            } else {
                "Enter = submit | Ctrl+H = toggle hints | Ctrl+S = solver | Ctrl+Q = quit"
            }
        } else {
            "Enter = submit | Ctrl+G = game | Ctrl+Z = undo | Ctrl+Q = quit"
        };

        f.render_widget(
            Paragraph::new(text).block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(border_color))
                    .title(format!("Input {} | {}", subtitle, help_text)),
            ),
            area,
        );
    }

    // Helper method that doesn't require &mut
    pub(in crate::ui) fn input_status_immutable(&self) -> InputStatus {
        use crate::solver::parse_pattern;

        if self.mode == GameMode::Game {
            let guess = self.input.trim();

            if guess.is_empty() {
                return InputStatus::Incomplete;
            }

            if guess.len() != self.solver.word_len() {
                return InputStatus::Invalid("guess length mismatch");
            }

            let guess_lower = guess.to_lowercase();

            if !self.allowed_lookup.contains(&guess_lower) {
                return InputStatus::Invalid("word not in allowed list");
            }

            return InputStatus::Valid;
        }

        let parts: Vec<_> = self.input.split_whitespace().collect();

        if parts.is_empty() {
            return InputStatus::Incomplete;
        }

        if parts.len() == 1 {
            return InputStatus::Incomplete;
        }

        if parts.len() > 2 {
            return InputStatus::Invalid("too many fields");
        }

        let guess = parts[0];
        let pattern = parts[1];

        if guess.len() != self.solver.word_len() {
            return InputStatus::Invalid("guess length mismatch");
        } else if !self.allowed_lookup.contains(&guess.to_lowercase()) {
            return InputStatus::Invalid("word not in allowed list");
        }

        if pattern.len() != self.solver.word_len() {
            return InputStatus::Invalid("pattern length mismatch");
        }

        if parse_pattern(pattern).is_err() {
            return InputStatus::Invalid("pattern must be G/Y/X");
        }

        InputStatus::Valid
    }
}
