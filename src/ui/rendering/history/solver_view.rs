use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Row, Table},
};

use crate::ui::App;

impl App {
    pub(in crate::ui) fn draw_solver_view(&self, f: &mut Frame, area: Rect) {
        if let Some(ref history_data) = self.history_data {
            let solver_stats = &history_data.solver_stats;

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(10), // Overall solver stats
                    Constraint::Length(8),  // Deviation analysis
                    Constraint::Min(5),     // Recent sessions
                ])
                .split(area);

            draw_solver_stats(f, chunks[0], solver_stats);
            draw_deviation_analysis(f, chunks[1], solver_stats);
            draw_recent_sessions(f, chunks[2], history_data);
        } else {
            let text = vec![
                Line::from(""),
                Line::from(Span::styled(
                    "No solver history available",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                )),
                Line::from(""),
                Line::from("Use solver mode (default mode) to track your solving sessions!"),
                Line::from(""),
                Line::from(""),
                Line::from(Span::styled(
                    "Controls:",
                    Style::default().add_modifier(Modifier::BOLD),
                )),
                Line::from("  Tab - Cycle Views"),
                Line::from("  Ctrl+R - Return to Solver Mode"),
                Line::from("  Ctrl+Q - Quit Application"),
            ];

            let paragraph = Paragraph::new(text).block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Solver History"),
            );

            f.render_widget(paragraph, area);
        }
    }
}

fn draw_solver_stats(f: &mut Frame, area: Rect, stats: &crate::ui::history::SolverStats) {
    let avg_guesses_str = if stats.completed_sessions > 0 {
        format!("{:.2}", stats.average_guesses)
    } else {
        "N/A".to_string()
    };

    let avg_entropy_str = format!("{:.2}", stats.average_entropy);
    let adherence_str = format!("{:.1}%", stats.optimal_adherence);
    let deviation_str = format!("{:.2}", stats.average_deviation);

    let lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::raw("  Total Sessions: "),
            Span::styled(
                format!("{}", stats.total_sessions),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("  |  Completed: "),
            Span::styled(
                format!("{}", stats.completed_sessions),
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("  |  Abandoned: "),
            Span::styled(
                format!("{}", stats.abandoned_sessions),
                Style::default()
                    .fg(Color::Gray)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::raw("  Avg Guesses: "),
            Span::styled(
                avg_guesses_str,
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("  |  Avg Entropy: "),
            Span::styled(
                avg_entropy_str,
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::raw("  Optimal Adherence: "),
            Span::styled(
                adherence_str,
                Style::default()
                    .fg(if stats.optimal_adherence >= 80.0 {
                        Color::Green
                    } else if stats.optimal_adherence >= 50.0 {
                        Color::Yellow
                    } else {
                        Color::Red
                    })
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("  |  Avg Deviation: "),
            Span::styled(
                deviation_str,
                Style::default()
                    .fg(if stats.average_deviation >= -0.1 {
                        Color::Green
                    } else if stats.average_deviation >= -0.5 {
                        Color::Yellow
                    } else {
                        Color::Red
                    })
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
    ];

    let paragraph = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Solver Statistics | Tab: Views | Ctrl+R: Exit"),
    );

    f.render_widget(paragraph, area);
}

fn draw_deviation_analysis(f: &mut Frame, area: Rect, stats: &crate::ui::history::SolverStats) {
    let bar_width = if stats.optimal_adherence > 0.0 {
        ((stats.optimal_adherence / 100.0) * 50.0) as usize
    } else {
        0
    };

    let optimal_bar = "█".repeat(bar_width);
    let deviation_bar = "█".repeat(50 - bar_width);

    let lines = vec![
        Line::from(""),
        Line::from("  Path Adherence:"),
        Line::from(""),
        Line::from(vec![
            Span::raw("  Optimal: "),
            Span::styled(optimal_bar, Style::default().fg(Color::Green)),
            Span::raw(format!(" {:.1}%", stats.optimal_adherence)),
        ]),
        Line::from(vec![
            Span::raw("  Deviated: "),
            Span::styled(deviation_bar, Style::default().fg(Color::Red)),
            Span::raw(format!(" {:.1}%", 100.0 - stats.optimal_adherence)),
        ]),
        Line::from(""),
    ];

    let paragraph = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Optimal Path Analysis"),
    );

    f.render_widget(paragraph, area);
}

fn draw_recent_sessions(f: &mut Frame, area: Rect, history_data: &crate::ui::history::HistoryData) {
    let recent_count = 10.min(history_data.solver_sessions.len());
    let recent_sessions = if recent_count > 0 {
        &history_data.solver_sessions[history_data.solver_sessions.len() - recent_count..]
    } else {
        &[]
    };

    let rows: Vec<Row> = recent_sessions
        .iter()
        .rev()
        .map(|session| {
            let date = session.timestamp.format("%Y-%m-%d %H:%M").to_string();
            let guesses = session.guess_count().to_string();
            let adherence = format!("{:.1}%", session.optimal_adherence());
            let avg_entropy = format!("{:.2}", session.average_entropy());
            let deviation = format!("{:.2}", session.average_deviation());
            let outcome = match session.outcome {
                crate::ui::history::SolverOutcome::Completed { .. } => "Completed",
                crate::ui::history::SolverOutcome::Abandoned => "Abandoned",
            };

            let outcome_style = match session.outcome {
                crate::ui::history::SolverOutcome::Completed { .. } => {
                    Style::default().fg(Color::Green)
                }
                crate::ui::history::SolverOutcome::Abandoned => Style::default().fg(Color::Gray),
            };

            Row::new(vec![
                date,
                guesses,
                adherence,
                avg_entropy,
                deviation,
                outcome.to_string(),
            ])
            .style(outcome_style)
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Length(16), // Date
            Constraint::Length(8),  // Guesses
            Constraint::Length(12), // Adherence
            Constraint::Length(10), // Avg Entropy
            Constraint::Length(10), // Deviation
            Constraint::Length(10), // Outcome
        ],
    )
    .header(
        Row::new(vec![
            "Date",
            "Guesses",
            "Adherence",
            "Entropy",
            "Deviation",
            "Status",
        ])
        .style(Style::default().add_modifier(Modifier::BOLD))
        .bottom_margin(1),
    )
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title("Recent Sessions (Latest 10)"),
    );

    f.render_widget(table, area);
}
