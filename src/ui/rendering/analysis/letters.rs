//! Letter frequency analysis rendering.

use ratatui::{
    Frame,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use crate::ui::app::App;

impl App {
    pub(in crate::ui) fn draw_letter_analysis(&self, f: &mut Frame, area: ratatui::layout::Rect) {
        if let Some(analysis) = &self.letter_analysis {
            let mut freq: Vec<(char, usize)> =
                analysis.frequencies.iter().map(|(c, v)| (*c, *v)).collect();

            // Sort by frequency descending
            freq.sort_by(|a, b| b.1.cmp(&a.1));

            let max_bar = area.width.saturating_sub(8) as usize;

            let mut lines = vec![
                Line::from(format!("Remaining: {} words", analysis.total_words)),
                Line::from(""),
            ];

            for (c, count) in freq.into_iter().take(10) {
                let width = if analysis.max_frequency > 0 {
                    (count * max_bar / analysis.max_frequency).max(1)
                } else {
                    0
                };

                let bar = "█".repeat(width);

                lines.push(Line::from(vec![
                    Span::raw(format!("{} {:>4} ", c, count)),
                    Span::styled(bar, Style::default().fg(Color::Cyan)),
                ]));
            }

            f.render_widget(
                Paragraph::new(lines)
                    .block(Block::default().borders(Borders::ALL).title("Letters")),
                area,
            );
        }
    }
}
