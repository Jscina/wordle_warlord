//! Guess history rendering with colored feedback.

use ratatui::{
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::{solver::Feedback, ui::app::App};

impl App {
    pub(in crate::ui) fn draw_guesses(&self, f: &mut Frame, area: ratatui::layout::Rect) {
        let lines: Vec<Line> = self
            .solver
            .guesses()
            .iter()
            .map(|g| {
                let spans: Vec<Span> = g
                    .word
                    .chars()
                    .zip(g.feedback.iter())
                    .map(|(c, fb)| {
                        let style = match fb {
                            Feedback::Green => Style::default().bg(Color::Green).fg(Color::Black),
                            Feedback::Yellow => Style::default().bg(Color::Yellow).fg(Color::Black),
                            Feedback::Gray => Style::default().bg(Color::DarkGray).fg(Color::White),
                        };
                        Span::styled(format!(" {} ", c.to_ascii_uppercase()), style)
                    })
                    .collect();
                Line::from(spans)
            })
            .collect();

        f.render_widget(
            Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title("Guesses")),
            area,
        );
    }
}
