//! Game list view rendering for history mode.

use ratatui::{
    layout::{Constraint, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Row, Table},
    Frame,
};

use crate::ui::App;

impl App {
    pub(in crate::ui) fn draw_list_view(&self, f: &mut Frame, area: Rect) {
        if let Some(ref history_data) = self.history_data {
            let games = history_data.games_for_page(self.history_page);
            let total_pages = history_data.total_pages();

            if games.is_empty() {
                let text = vec![
                    Line::from(""),
                    Line::from(Span::styled(
                        "No games found",
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    )),
                ];

                let paragraph = Paragraph::new(text)
                    .block(Block::default().borders(Borders::ALL).title("Game History"));

                f.render_widget(paragraph, area);
                return;
            }

            // Create table rows
            let start_index = self.history_page * 10;
            let rows: Vec<Row> = games
                .iter()
                .enumerate()
                .map(|(page_idx, game)| {
                    let date = game.timestamp.format("%Y-%m-%d %H:%M").to_string();
                    let guesses = game.guess_count().to_string();
                    let outcome = match game.outcome {
                        crate::ui::history::GameOutcome::Won { .. } => "Won",
                        crate::ui::history::GameOutcome::Lost => "Lost",
                        crate::ui::history::GameOutcome::Abandoned => "Abandoned",
                    };

                    let outcome_style = match game.outcome {
                        crate::ui::history::GameOutcome::Won { .. } => {
                            Style::default().fg(Color::Green)
                        }
                        crate::ui::history::GameOutcome::Lost => Style::default().fg(Color::Red),
                        crate::ui::history::GameOutcome::Abandoned => {
                            Style::default().fg(Color::Gray)
                        }
                    };

                    // Show number for selection (1-10)
                    let num = format!("{}.", page_idx + 1);

                    Row::new(vec![
                        num,
                        date,
                        game.target_word.clone(),
                        guesses,
                        outcome.to_string(),
                    ])
                    .style(outcome_style)
                })
                .collect();

            // Create title with page info and instructions
            let title = format!(
                "Game History - Page {}/{} (Showing {}-{} of {}) | PgUp/PgDn: Navigate | 1-9: View Detail | Tab: Views | Esc: Stats | Ctrl+R: Exit",
                self.history_page + 1,
                total_pages,
                start_index + 1,
                start_index + games.len(),
                history_data.games.len()
            );

            let table = Table::new(
                rows,
                [
                    Constraint::Length(3),  // Number
                    Constraint::Length(16), // Date
                    Constraint::Length(10), // Word
                    Constraint::Length(8),  // Guesses
                    Constraint::Length(12), // Outcome
                ],
            )
            .header(
                Row::new(vec!["#", "Date", "Word", "Guesses", "Result"])
                    .style(Style::default().add_modifier(Modifier::BOLD))
                    .bottom_margin(1),
            )
            .block(Block::default().borders(Borders::ALL).title(title));

            f.render_widget(table, area);
        } else {
            let text = vec![
                Line::from(""),
                Line::from(Span::styled(
                    "No history loaded",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                )),
            ];

            let paragraph = Paragraph::new(text)
                .block(Block::default().borders(Borders::ALL).title("Game History"));

            f.render_widget(paragraph, area);
        }
    }
}
