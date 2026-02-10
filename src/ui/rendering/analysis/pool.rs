//! Solution pool statistics and entropy graph rendering.

use ratatui::{
    text::Line,
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::ui::app::App;

impl App {
    pub(in crate::ui) fn draw_solution_pool(&self, f: &mut Frame, area: ratatui::layout::Rect) {
        if let Some(stats) = &self.solution_pool_stats {
            let mut lines = vec![
                Line::from("Solution Pool"),
                Line::from(format!("Total: {} remaining", stats.total_remaining)),
                Line::from(format!(
                    "Filtered: {:.1}% eliminated",
                    stats.eliminated_percentage
                )),
                Line::from(format!("Entropy: {:.2} bits", stats.entropy)),
                Line::from(""),
            ];

            if !self.entropy_history.is_empty() {
                let graph_height = area.height.saturating_sub(lines.len() as u16 + 2) as usize;
                let graph_width = area.width.saturating_sub(2) as usize;

                if graph_height > 0 && graph_width > 0 {
                    let start = self.entropy_history.len().saturating_sub(graph_width);
                    let slice = &self.entropy_history[start..];

                    let max_entropy = slice.iter().cloned().fold(0.0_f64, f64::max);

                    let mut grid = vec![vec![' '; graph_width]; graph_height];

                    for (i, value) in slice.iter().enumerate() {
                        let x = i;

                        let normalized = if max_entropy > 0.0 {
                            ((value + 1.0).ln() / (max_entropy + 1.0).ln()).clamp(0.0, 1.0)
                        } else {
                            0.0
                        };

                        let y =
                            graph_height - 1 - (normalized * (graph_height as f64 - 1.0)) as usize;

                        grid[y][x] = '●';
                    }

                    for row in grid {
                        lines.push(Line::from(row.into_iter().collect::<String>()));
                    }
                }
            }

            f.render_widget(
                Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title("Pool")),
                area,
            );
        }
    }
}
