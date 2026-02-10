//! Single game detail view rendering for history mode.

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::{solver::Feedback, ui::App};

impl App {
    pub(in crate::ui) fn draw_detail_view(&self, f: &mut Frame, area: Rect) {
        if let Some(ref history_data) = self.history_data {
            if let Some(game) = history_data.selected_game() {
                // Split the area
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(6), // Game header
                        Constraint::Min(10),   // Guesses
                    ])
                    .split(area);

                // Draw game header
                draw_game_header(f, chunks[0], game);

                // Draw guesses
                draw_game_guesses(f, chunks[1], game);
            } else {
                let text = vec![
                    Line::from(""),
                    Line::from(Span::styled(
                        "No game selected",
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    )),
                ];

                let paragraph = Paragraph::new(text)
                    .block(Block::default().borders(Borders::ALL).title("Game Detail"));

                f.render_widget(paragraph, area);
            }
        }
    }
}

fn draw_game_header(f: &mut Frame, area: Rect, game: &crate::ui::history::GameRecord) {
    let date = game.timestamp.format("%Y-%m-%d %H:%M:%S").to_string();
    let outcome_text = match game.outcome {
        crate::ui::history::GameOutcome::Won { guesses } => {
            format!("Won in {} guess(es)", guesses)
        }
        crate::ui::history::GameOutcome::Lost => "Lost (ran out of guesses)".to_string(),
        crate::ui::history::GameOutcome::Abandoned => "Abandoned (incomplete)".to_string(),
    };

    let outcome_color = match game.outcome {
        crate::ui::history::GameOutcome::Won { .. } => Color::Green,
        crate::ui::history::GameOutcome::Lost => Color::Red,
        crate::ui::history::GameOutcome::Abandoned => Color::Gray,
    };

    let lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::raw("  Date: "),
            Span::styled(
                date,
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::raw("  Target Word: "),
            Span::styled(
                game.target_word.to_uppercase(),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("  |  Outcome: "),
            Span::styled(
                outcome_text,
                Style::default()
                    .fg(outcome_color)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
    ];

    let paragraph = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Game Details | Esc: Back to List"),
    );

    f.render_widget(paragraph, area);
}

fn draw_game_guesses(f: &mut Frame, area: Rect, game: &crate::ui::history::GameRecord) {
    let mut lines = vec![Line::from("")];

    for (i, guess) in game.guesses.iter().enumerate() {
        // Build the colored guess display
        let mut spans = vec![Span::raw(format!("  {}. ", i + 1))];

        for (ch, feedback) in guess.word.chars().zip(&guess.feedback) {
            let color = match feedback {
                Feedback::Green => Color::Green,
                Feedback::Yellow => Color::Yellow,
                Feedback::Gray => Color::DarkGray,
            };

            spans.push(Span::styled(
                format!(" {} ", ch.to_uppercase()),
                Style::default()
                    .fg(Color::Black)
                    .bg(color)
                    .add_modifier(Modifier::BOLD),
            ));
        }

        lines.push(Line::from(spans));
        lines.push(Line::from(""));
    }

    if game.guesses.is_empty() {
        lines.push(Line::from(Span::styled(
            "  No guesses recorded",
            Style::default().fg(Color::Gray),
        )));
    }

    let paragraph =
        Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title("Guesses"));

    f.render_widget(paragraph, area);
}
