use ratatui::{
    Frame,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use crate::ui::app::App;

impl App {
    pub(in crate::ui) fn draw_constraint_summary(
        &self,
        f: &mut Frame,
        area: ratatui::layout::Rect,
    ) {
        if let Some(summary) = &self.constraint_summary {
            let mut lines = vec![Line::from("Active Constraints")];

            // Greens with guess source
            for (letter, pos, guess) in &summary.greens {
                lines.push(Line::from(vec![
                    Span::styled("✓ ", Style::default().fg(Color::Green)),
                    Span::raw(format!(
                        "{}({}) from '{}'",
                        letter,
                        pos + 1,
                        guess.to_uppercase()
                    )),
                ]));
            }

            // Yellows with guess source
            for (letter, positions, guess) in &summary.yellows {
                let pos_str: String = positions
                    .iter()
                    .map(|p| (p + 1).to_string())
                    .collect::<Vec<_>>()
                    .join(",");
                lines.push(Line::from(vec![
                    Span::styled("✓ ", Style::default().fg(Color::Yellow)),
                    Span::raw(format!(
                        "{}({}) from '{}'",
                        letter,
                        pos_str,
                        guess.to_uppercase()
                    )),
                ]));
            }

            // Grays
            if !summary.grays.is_empty() {
                let gray_str: String = summary
                    .grays
                    .iter()
                    .map(|c| c.to_string())
                    .collect::<Vec<String>>()
                    .join(" ");
                lines.push(Line::from(vec![
                    Span::styled("✗ ", Style::default().fg(Color::DarkGray)),
                    Span::raw(gray_str),
                ]));
            }

            f.render_widget(
                Paragraph::new(lines)
                    .block(Block::default().borders(Borders::ALL).title("Constraints")),
                area,
            );
        }
    }
}
