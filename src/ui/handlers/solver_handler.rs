//! Solver mode state management and analysis computation.

use crate::{
    analysis::{
        compute_constraint_summary, compute_letter_analysis, compute_position_analysis,
        compute_solution_pool_stats,
    },
    scoring::score_and_sort,
    solver::SolverState,
};

use super::super::app::App;

/// Helper struct for managing solver-specific state and analysis.
pub struct SolverHandler<'a> {
    app: &'a mut App,
}

impl<'a> SolverHandler<'a> {
    pub fn new(app: &'a mut App) -> Self {
        Self { app }
    }

    pub fn recompute(&mut self) {
        let remaining = self.app.solver.filter(&self.app.solution_words);

        if self.app.solver.guesses().is_empty() {
            self.app.suggestions.clear();
        } else {
            self.app.suggestions = score_and_sort(&remaining, &self.app.allowed_lookup);
        }

        self.app.analysis_dirty = true;
    }

    pub fn recompute_analysis(&mut self) {
        if !self.app.analysis_dirty {
            return;
        }

        let remaining = self.app.solver.filter(&self.app.solution_words);

        self.app.letter_analysis = Some(compute_letter_analysis(&remaining));
        tracing::info!("LetterAnalysis: {:?}", self.app.letter_analysis);
        self.app.position_analysis = Some(compute_position_analysis(&remaining, &self.app.solver));
        tracing::info!("PositionAnalysis: {:?}", self.app.position_analysis);
        self.app.constraint_summary = Some(compute_constraint_summary(&self.app.solver));
        tracing::info!("ConstraintSummary: {:?}", self.app.constraint_summary);
        self.app.solution_pool_stats = Some(compute_solution_pool_stats(
            &self.app.solution_words,
            &remaining,
        ));

        tracing::info!("SolutionPoolStats: {:?}", self.app.solution_pool_stats);
        if let Some(stats) = &self.app.solution_pool_stats {
            // Only push if not rebuilding (i.e., during normal guess submission)
            if self.app.entropy_history.len() < self.app.solver.guesses().len() {
                self.app.entropy_history.push(stats.entropy);
            }
        }

        self.app.analysis_dirty = false;
    }

    pub fn undo_guess(&mut self) {
        if !self.app.solver.guesses().is_empty() {
            if self.app.solver_session_active && !self.app.solver_session_paused {
                // Log undo in solver session
                let last_guess = self.app.solver.guesses().last().unwrap();
                tracing::info!("Solver undo: removed guess {}", last_guess.word);
            }
            self.app.solver.pop_guess();
            self.recompute();
            self.rebuild_entropy_history();
            self.app.analysis_dirty = true;
        }
    }

    pub fn rebuild_entropy_history(&mut self) {
        self.app.entropy_history.clear();
        let guesses = self.app.solver.guesses();
        let mut temp_solver = SolverState::new(self.app.solver.word_len());
        for guess in guesses {
            temp_solver.add_guess(guess.clone());
            let remaining = temp_solver.filter(&self.app.solution_words);
            let stats = compute_solution_pool_stats(&self.app.solution_words, &remaining);
            self.app.entropy_history.push(stats.entropy);
        }
    }
}
