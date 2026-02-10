//! Statistics dashboard rendering for history mode.

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Row, Table},
    Frame,
};

use crate::ui::App;

impl App {
    pub(in crate::ui) fn draw_stats_view(&self, f: &mut Frame, area: Rect) {
        if let Some(ref history_data) = self.history_data {
            let stats = &history_data.stats;

            // Split the area into sections
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(8),  // Overall stats
                    Constraint::Length(10), // Guess distribution
                    Constraint::Min(5),     // Recent games
                ])
                .split(area);

            // Draw overall statistics with help text
            draw_overall_stats(f, chunks[0], stats);

            // Draw guess distribution
            draw_guess_distribution(f, chunks[1], stats);

            // Draw recent games
            draw_recent_games(f, chunks[2], history_data);
        } else {
            // No history loaded
            let text = vec![
                Line::from(""),
                Line::from(Span::styled(
                    "No game history available",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                )),
                Line::from(""),
                Line::from("Play some games first!"),
                Line::from(""),
                Line::from(""),
                Line::from(Span::styled(
                    "Controls:",
                    Style::default().add_modifier(Modifier::BOLD),
                )),
                Line::from("  Ctrl+R - Return to Solver Mode"),
                Line::from("  Ctrl+Q - Quit Application"),
            ];

            let paragraph = Paragraph::new(text)
                .block(Block::default().borders(Borders::ALL).title("Game History"));

            f.render_widget(paragraph, area);
        }
    }
}

fn draw_overall_stats(f: &mut Frame, area: Rect, stats: &crate::ui::history::HistoryStats) {
    let win_rate_str = format!("{:.1}%", stats.win_rate);
    let avg_guesses_str = if stats.wins > 0 {
        format!("{:.2}", stats.average_guesses)
    } else {
        "N/A".to_string()
    };

    let streak_text = if stats.current_streak > 0 {
        format!("{} wins", stats.current_streak)
    } else if stats.current_streak < 0 {
        format!("{} losses", -stats.current_streak)
    } else {
        "0".to_string()
    };

    let lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::raw("  Games Played: "),
            Span::styled(
                format!("{}", stats.total_games),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("  |  Wins: "),
            Span::styled(
                format!("{}", stats.wins),
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("  |  Losses: "),
            Span::styled(
                format!("{}", stats.losses + stats.abandoned),
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::raw("  Win Rate: "),
            Span::styled(
                win_rate_str,
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("  |  Avg Guesses: "),
            Span::styled(
                avg_guesses_str,
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("  |  Current Streak: "),
            Span::styled(
                streak_text,
                Style::default()
                    .fg(if stats.current_streak > 0 {
                        Color::Green
                    } else if stats.current_streak < 0 {
                        Color::Red
                    } else {
                        Color::Gray
                    })
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("  |  Best Streak: "),
            Span::styled(
                format!("{}", stats.best_win_streak),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
    ];

    let paragraph = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Statistics | Tab: List View | Ctrl+R: Exit | Ctrl+Q: Quit"),
    );

    f.render_widget(paragraph, area);
}

fn draw_guess_distribution(f: &mut Frame, area: Rect, stats: &crate::ui::history::HistoryStats) {
    let max_count = *stats.guess_distribution.iter().max().unwrap_or(&1);

    let mut lines = vec![Line::from("")];

    for (i, count) in stats.guess_distribution.iter().enumerate() {
        let guess_num = i + 1;
        let bar_width = if max_count > 0 {
            ((*count as f64 / max_count as f64) * 40.0) as usize
        } else {
            0
        };

        let bar = "█".repeat(bar_width);

        lines.push(Line::from(vec![
            Span::raw(format!("  {} ", guess_num)),
            Span::styled(bar, Style::default().fg(Color::Green)),
            Span::raw(format!(" {}", count)),
        ]));
    }

    lines.push(Line::from(""));

    let paragraph = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Guess Distribution"),
    );

    f.render_widget(paragraph, area);
}

fn draw_recent_games(f: &mut Frame, area: Rect, history_data: &crate::ui::history::HistoryData) {
    let recent_count = 10.min(history_data.games.len());
    let recent_games = if recent_count > 0 {
        &history_data.games[history_data.games.len() - recent_count..]
    } else {
        &[]
    };

    let rows: Vec<Row> = recent_games
        .iter()
        .rev()
        .map(|game| {
            let date = game.timestamp.format("%Y-%m-%d %H:%M").to_string();
            let outcome = match game.outcome {
                crate::ui::history::GameOutcome::Won { guesses } => {
                    format!("Won in {}", guesses)
                }
                crate::ui::history::GameOutcome::Lost => "Lost".to_string(),
                crate::ui::history::GameOutcome::Abandoned => "Abandoned".to_string(),
            };

            let outcome_style = match game.outcome {
                crate::ui::history::GameOutcome::Won { .. } => Style::default().fg(Color::Green),
                crate::ui::history::GameOutcome::Lost
                | crate::ui::history::GameOutcome::Abandoned => Style::default().fg(Color::Red),
            };

            Row::new(vec![date, game.target_word.clone(), outcome]).style(outcome_style)
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Length(16),
            Constraint::Length(10),
            Constraint::Length(15),
        ],
    )
    .header(
        Row::new(vec!["Date", "Word", "Result"])
            .style(Style::default().add_modifier(Modifier::BOLD))
            .bottom_margin(1),
    )
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title("Recent Games (Latest 10)"),
    );

    f.render_widget(table, area);
}
