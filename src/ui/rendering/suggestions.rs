use ratatui::{
    Frame,
    widgets::{Block, Borders, List, ListItem},
};

use crate::ui::app::App;

impl App {
    pub(in crate::ui) fn draw_suggestions(&self, f: &mut Frame, area: ratatui::layout::Rect) {
        let items: Vec<ListItem> = if self.suggestions.is_empty() {
            vec![ListItem::new("No suggestions yet")]
        } else {
            self.suggestions
                .iter()
                .take(10)
                .map(|(w, s)| ListItem::new(format!("{w} ({s})")))
                .collect()
        };

        let title = format!("Suggestions (remaining: {})", self.suggestions.len());

        f.render_widget(
            List::new(items).block(Block::default().borders(Borders::ALL).title(title)),
            area,
        );
    }
}
