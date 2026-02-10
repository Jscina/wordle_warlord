//! Position-based letter analysis rendering.

use ratatui::{
    text::Line,
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::ui::app::App;

impl App {
    pub(in crate::ui) fn draw_position_analysis(&self, f: &mut Frame, area: ratatui::layout::Rect) {
        if let Some(analysis) = &self.position_analysis {
            let mut lines = vec![Line::from("Position Analysis"), Line::from("")];

            for (pos, letters) in analysis.possible_letters.iter().enumerate() {
                let letters_with_freq: Vec<String> = letters
                    .iter()
                    .map(|c| {
                        if let Some(&count) = analysis.position_frequencies[pos].get(c) {
                            format!("{}({})", c, count)
                        } else {
                            c.to_string()
                        }
                    })
                    .collect();

                let letters_str = letters_with_freq.join(" ");

                lines.push(Line::from(format!("Pos {}: [{}]", pos + 1, letters_str)));
            }

            f.render_widget(
                Paragraph::new(lines)
                    .block(Block::default().borders(Borders::ALL).title("Positions")),
                area,
            );
        }
    }
}
