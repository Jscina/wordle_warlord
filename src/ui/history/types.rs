//! Data structures for game history tracking.

use chrono::{DateTime, Utc};

use crate::solver::Feedback;

use super::solver_types::{SolverSession, SolverStats};

/// Outcome of a completed or abandoned game.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GameOutcome {
    Won { guesses: usize },
    Lost,
    Abandoned,
}

/// A single guess within a game.
#[derive(Debug, Clone)]
pub struct GameGuess {
    pub word: String,
    pub feedback: Vec<Feedback>,
}

/// A complete game record parsed from logs.
#[derive(Debug, Clone)]
pub struct GameRecord {
    pub timestamp: DateTime<Utc>,
    pub target_word: String,
    pub guesses: Vec<GameGuess>,
    pub outcome: GameOutcome,
}

impl GameRecord {
    /// Returns the number of guesses made in this game.
    pub fn guess_count(&self) -> usize {
        self.guesses.len()
    }

    /// Returns true if the game was lost.
    pub fn was_lost(&self) -> bool {
        matches!(self.outcome, GameOutcome::Lost)
    }
}

/// Aggregated statistics across all games.
#[derive(Debug, Clone, Default)]
pub struct HistoryStats {
    pub total_games: usize,
    pub wins: usize,
    pub losses: usize,
    pub abandoned: usize,
    pub win_rate: f64,
    pub average_guesses: f64,
    pub current_streak: i32, // Positive for wins, negative for losses
    pub best_win_streak: usize,
    pub guess_distribution: [usize; 6], // Count of wins by guess number (1-6)
}

impl HistoryStats {
    /// Compute statistics from a list of game records.
    pub fn from_games(games: &[GameRecord]) -> Self {
        if games.is_empty() {
            return Self::default();
        }

        let mut stats = Self::default();
        stats.total_games = games.len();

        let mut total_guesses_for_wins = 0;
        let mut current_streak = 0;
        let mut best_win_streak = 0;
        let mut current_win_streak = 0;

        for game in games {
            match game.outcome {
                GameOutcome::Won { guesses } => {
                    stats.wins += 1;
                    total_guesses_for_wins += guesses;

                    // Update guess distribution (1-indexed to 0-indexed)
                    if guesses >= 1 && guesses <= 6 {
                        stats.guess_distribution[guesses - 1] += 1;
                    }

                    // Update streaks
                    if current_streak >= 0 {
                        current_streak += 1;
                    } else {
                        current_streak = 1;
                    }
                    current_win_streak += 1;
                    best_win_streak = best_win_streak.max(current_win_streak);
                }
                GameOutcome::Lost | GameOutcome::Abandoned => {
                    if game.was_lost() {
                        stats.losses += 1;
                    } else {
                        stats.abandoned += 1;
                    }

                    // Update streaks
                    if current_streak <= 0 {
                        current_streak -= 1;
                    } else {
                        current_streak = -1;
                    }
                    current_win_streak = 0;
                }
            }
        }

        stats.current_streak = current_streak;
        stats.best_win_streak = best_win_streak;

        // Calculate win rate excluding abandoned games
        let completed_games = stats.wins + stats.losses;
        if completed_games > 0 {
            stats.win_rate = (stats.wins as f64 / completed_games as f64) * 100.0;
        }

        if stats.wins > 0 {
            stats.average_guesses = total_guesses_for_wins as f64 / stats.wins as f64;
        }

        stats
    }
}

/// Display mode for history viewer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HistoryViewMode {
    Stats,  // Statistics dashboard
    List,   // Paginated game list
    Detail, // Single game detail view
    Solver, // Solver statistics view
}

/// Container for all history data.
#[derive(Debug, Clone)]
pub struct HistoryData {
    pub games: Vec<GameRecord>,
    pub stats: HistoryStats,
    pub solver_sessions: Vec<SolverSession>,
    pub solver_stats: SolverStats,
    pub selected_game_index: Option<usize>,
}

impl HistoryData {
    pub fn new(games: Vec<GameRecord>, sessions: Vec<SolverSession>) -> Self {
        let stats = HistoryStats::from_games(&games);
        let solver_stats = SolverStats::from_sessions(&sessions);
        Self {
            games,
            stats,
            solver_sessions: sessions,
            solver_stats,
            selected_game_index: None,
        }
    }

    /// Get the total number of pages for pagination (10 games per page).
    pub fn total_pages(&self) -> usize {
        if self.games.is_empty() {
            1
        } else {
            (self.games.len() + 9) / 10 // Ceiling division
        }
    }

    /// Get games for a specific page (0-indexed).
    pub fn games_for_page(&self, page: usize) -> &[GameRecord] {
        let start = page * 10;
        let end = (start + 10).min(self.games.len());
        if start >= self.games.len() {
            &[]
        } else {
            &self.games[start..end]
        }
    }

    /// Get the currently selected game, if any.
    pub fn selected_game(&self) -> Option<&GameRecord> {
        self.selected_game_index.and_then(|idx| self.games.get(idx))
    }

    /// Select a game by its index in the games list.
    pub fn select_game(&mut self, index: usize) {
        if index < self.games.len() {
            self.selected_game_index = Some(index);
        }
    }

    /// Clear the game selection.
    pub fn clear_selection(&mut self) {
        self.selected_game_index = None;
    }
}
