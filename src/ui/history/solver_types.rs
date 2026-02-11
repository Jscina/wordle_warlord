//! Data structures for solver session tracking.

use chrono::{DateTime, Utc};

/// A single guess within a solver session
#[derive(Debug, Clone)]
pub struct SolverGuess {
    #[allow(dead_code)]
    pub word: String,
    #[allow(dead_code)]
    pub pool_size_before: usize,
    #[allow(dead_code)]
    pub pool_size_after: usize,
    pub entropy: f64,
    #[allow(dead_code)]
    pub optimal_word: String, // Best choice at this step
    #[allow(dead_code)]
    pub optimal_entropy: f64, // Entropy of optimal choice
    pub deviation_score: f64, // Entropy difference (actual - optimal)
}

impl SolverGuess {
    /// Returns true if this guess was optimal (or within 0.01 of optimal)
    pub fn was_optimal(&self) -> bool {
        self.deviation_score >= -0.01
    }
}

/// Outcome of a solver session
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SolverOutcome {
    Completed { guesses: usize },
    Abandoned,
}

/// A complete solver session record
#[derive(Debug, Clone)]
pub struct SolverSession {
    pub timestamp: DateTime<Utc>,
    pub guesses: Vec<SolverGuess>,
    pub outcome: SolverOutcome,
}

impl SolverSession {
    /// Returns the number of guesses made in this session
    pub fn guess_count(&self) -> usize {
        self.guesses.len()
    }

    /// Returns the percentage of guesses that were optimal
    pub fn optimal_adherence(&self) -> f64 {
        if self.guesses.is_empty() {
            return 100.0;
        }
        let optimal_count = self.guesses.iter().filter(|g| g.was_optimal()).count();
        (optimal_count as f64 / self.guesses.len() as f64) * 100.0
    }

    /// Returns the average deviation from optimal path
    pub fn average_deviation(&self) -> f64 {
        if self.guesses.is_empty() {
            return 0.0;
        }
        let total: f64 = self.guesses.iter().map(|g| g.deviation_score).sum();
        total / self.guesses.len() as f64
    }

    /// Returns the average entropy per guess
    pub fn average_entropy(&self) -> f64 {
        if self.guesses.is_empty() {
            return 0.0;
        }
        let total: f64 = self.guesses.iter().map(|g| g.entropy).sum();
        total / self.guesses.len() as f64
    }
}

/// Aggregated statistics for solver sessions
#[derive(Debug, Clone, Default)]
pub struct SolverStats {
    pub total_sessions: usize,
    pub completed_sessions: usize,
    pub abandoned_sessions: usize,
    pub average_guesses: f64,
    pub average_entropy: f64,
    pub optimal_adherence: f64, // % of all guesses that were optimal
    pub average_deviation: f64, // Average entropy loss from optimal
}

impl SolverStats {
    /// Compute statistics from a list of solver sessions
    pub fn from_sessions(sessions: &[SolverSession]) -> Self {
        if sessions.is_empty() {
            return Self::default();
        }

        let mut stats = Self {
            total_sessions: sessions.len(),
            ..Default::default()
        };

        let mut total_guesses = 0;
        let mut total_entropy = 0.0;
        let mut total_optimal_guesses = 0;
        let mut total_deviation = 0.0;
        let mut all_guess_count = 0;

        for session in sessions {
            match session.outcome {
                SolverOutcome::Completed { guesses } => {
                    stats.completed_sessions += 1;
                    total_guesses += guesses;
                }
                SolverOutcome::Abandoned => {
                    stats.abandoned_sessions += 1;
                }
            }

            // Count optimal guesses and deviations across all sessions
            for guess in &session.guesses {
                all_guess_count += 1;
                total_entropy += guess.entropy;
                total_deviation += guess.deviation_score;
                if guess.was_optimal() {
                    total_optimal_guesses += 1;
                }
            }
        }

        if stats.completed_sessions > 0 {
            stats.average_guesses = total_guesses as f64 / stats.completed_sessions as f64;
        }

        if all_guess_count > 0 {
            stats.average_entropy = total_entropy / all_guess_count as f64;
            stats.optimal_adherence =
                (total_optimal_guesses as f64 / all_guess_count as f64) * 100.0;
            stats.average_deviation = total_deviation / all_guess_count as f64;
        }

        stats
    }
}
