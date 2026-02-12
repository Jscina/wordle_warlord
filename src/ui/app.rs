use std::{collections::HashSet, fmt::Display, io::Stdout};

use anyhow::Result;
use chrono::{DateTime, Utc};
use crossterm::event::{self, Event};
use ratatui::{backend::CrosstermBackend, Terminal};
use sqlx::SqlitePool;
use tracing::info;

use crate::{
    analysis::{ConstraintSummary, LetterAnalysis, PositionAnalysis, SolutionPoolStats},
    solver::SolverState,
};

use super::{
    history::{HistoryData, HistoryViewMode},
    types::{GameMode, LogBuffer},
};

/// Main application state container.
pub struct App {
    pub(in crate::ui) solution_words: Vec<String>,
    pub(in crate::ui) allowed_lookup: HashSet<String>,
    pub(in crate::ui) solver: SolverState,
    pub(in crate::ui) input: String,
    pub(in crate::ui) suggestions: Vec<(String, usize)>,
    pub(in crate::ui) mode: GameMode,
    pub(in crate::ui) target_word: Option<String>,
    pub(in crate::ui) remaining_guesses: usize,
    pub(in crate::ui) game_won: bool,
    pub(in crate::ui) game_over: bool,
    pub(in crate::ui) show_suggestions: bool,
    pub(in crate::ui) show_analysis: bool,
    pub(in crate::ui) letter_analysis: Option<LetterAnalysis>,
    pub(in crate::ui) position_analysis: Option<PositionAnalysis>,
    pub(in crate::ui) constraint_summary: Option<ConstraintSummary>,
    pub(in crate::ui) solution_pool_stats: Option<SolutionPoolStats>,
    pub(in crate::ui) entropy_history: Vec<f64>,
    pub(in crate::ui) analysis_dirty: bool,
    pub(in crate::ui) logs: LogBuffer,
    pub(in crate::ui) history_data: Option<HistoryData>,
    pub(in crate::ui) history_view_mode: HistoryViewMode,
    pub(in crate::ui) history_page: usize,
    pub(in crate::ui) solver_session_active: bool,
    pub(in crate::ui) solver_session_start: Option<DateTime<Utc>>,
    pub(in crate::ui) solver_session_paused: bool,
    pub(in crate::ui) db_pool: SqlitePool,
    pub(in crate::ui) current_game_id: Option<i64>,
    pub(in crate::ui) current_session_id: Option<i64>,
}

impl App {
    pub fn new(
        words: Vec<String>,
        solution_words: Vec<String>,
        word_len: usize,
        logs: LogBuffer,
        db_pool: SqlitePool,
    ) -> Self {
        let allowed_lookup: HashSet<String> = words.iter().cloned().collect();

        Self {
            solution_words,
            allowed_lookup,
            solver: SolverState::new(word_len),
            input: String::new(),
            suggestions: Vec::new(),
            mode: GameMode::Solver,
            target_word: None,
            remaining_guesses: 6,
            game_won: false,
            game_over: false,
            show_suggestions: true,
            show_analysis: true,
            letter_analysis: None,
            position_analysis: None,
            constraint_summary: None,
            solution_pool_stats: None,
            entropy_history: Vec::new(),
            analysis_dirty: true,
            logs,
            history_data: None,
            history_view_mode: HistoryViewMode::Stats,
            history_page: 0,
            solver_session_active: true, // Start with session active since we're in Solver mode
            solver_session_start: Some(Utc::now()),
            solver_session_paused: false,
            db_pool,
            current_game_id: None,
            current_session_id: None,
        }
    }

    pub fn run(&mut self, terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
        info!("UI started");
        self.log("UI started");

        // Log solver session start (app starts in Solver mode)
        if self.solver_session_active {
            self.log("Solver session started");
        }

        loop {
            // Recompute analysis if needed
            super::handlers::SolverHandler::new(self).recompute_analysis();

            terminal.draw(|f| self.draw(f))?;

            let event = event::read()?;
            if let Event::Key(key) = event {
                // Use InputHandler to process keyboard input
                if super::handlers::InputHandler::new(self).handle_key(key) {
                    return Ok(());
                }
            }
        }
    }

    pub(in crate::ui) fn log(&self, msg: impl Into<String> + Display) {
        tracing::info!("{}", &msg);
        self.logs.push(msg.into());
    }

    /// Execute an async database operation from sync context
    pub(in crate::ui) fn run_db_operation<F, T>(&self, future: F) -> Result<T>
    where
        F: std::future::Future<Output = Result<T>>,
    {
        tokio::task::block_in_place(|| tokio::runtime::Handle::current().block_on(future))
    }
}
