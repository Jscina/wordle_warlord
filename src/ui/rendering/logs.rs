//! Log panel rendering.

use ratatui::{
    Frame,
    text::Line,
    widgets::{Block, Borders, Paragraph},
};

use crate::ui::app::App;

impl App {
    pub(in crate::ui) fn draw_logs(&self, f: &mut Frame, area: ratatui::layout::Rect) {
        let logs = self.logs.lines();

        let height = area.height as usize;
        let start = logs.len().saturating_sub(height);

        let lines: Vec<Line> = logs[start..]
            .iter()
            .map(|l| Line::from(l.clone()))
            .collect();

        f.render_widget(
            Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title("Logs")),
            area,
        );
    }
}
