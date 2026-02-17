use crate::solver::Feedback as SolverFeedback;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GameOutcome {
    Won,
    Lost,
    Abandoned,
}

impl std::fmt::Display for GameOutcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            GameOutcome::Won => "won",
            GameOutcome::Lost => "lost",
            GameOutcome::Abandoned => "abandoned",
        };
        write!(f, "{}", s)
    }
}

impl GameOutcome {
    pub fn from_string(s: &str) -> Option<Self> {
        match s {
            "won" => Some(GameOutcome::Won),
            "lost" => Some(GameOutcome::Lost),
            "abandoned" => Some(GameOutcome::Abandoned),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SolverOutcome {
    Completed,
    Abandoned,
}

impl std::fmt::Display for SolverOutcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            SolverOutcome::Completed => "completed",
            SolverOutcome::Abandoned => "abandoned",
        };
        write!(f, "{}", s)
    }
}

impl SolverOutcome {
    pub fn from_string(s: &str) -> Option<Self> {
        match s {
            "completed" => Some(SolverOutcome::Completed),
            "abandoned" => Some(SolverOutcome::Abandoned),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Feedback {
    Green,
    Yellow,
    Gray,
}

impl Feedback {
    pub fn to_string(&self) -> &'static str {
        match self {
            Feedback::Green => "green",
            Feedback::Yellow => "yellow",
            Feedback::Gray => "gray",
        }
    }

    pub fn from_string(s: &str) -> Option<Self> {
        match s {
            "green" => Some(Feedback::Green),
            "yellow" => Some(Feedback::Yellow),
            "gray" => Some(Feedback::Gray),
            _ => None,
        }
    }

    /// Convert from solver Feedback type
    pub fn from_solver(sf: &SolverFeedback) -> Self {
        match sf {
            SolverFeedback::Green => Feedback::Green,
            SolverFeedback::Yellow => Feedback::Yellow,
            SolverFeedback::Gray => Feedback::Gray,
        }
    }

    /// Convert to solver Feedback type
    pub fn to_solver(&self) -> SolverFeedback {
        match self {
            Feedback::Green => SolverFeedback::Green,
            Feedback::Yellow => SolverFeedback::Yellow,
            Feedback::Gray => SolverFeedback::Gray,
        }
    }
}

/// Represents a game record in the database
#[derive(Debug, Clone)]
pub struct Game {
    pub id: i64,
    pub timestamp: DateTime<Utc>,
    pub target_word: String,
    pub outcome: GameOutcome,
    pub guesses_count: i64,
}

/// Represents a single guess in a game
#[derive(Debug, Clone)]
pub struct GameGuess {
    pub id: i64,
    pub game_id: i64,
    pub guess_number: i64,
    pub word: String,
    pub feedback: Vec<Feedback>,
}

/// Represents a solver session in the database
#[derive(Debug, Clone)]
pub struct SolverSession {
    pub id: i64,
    pub timestamp: DateTime<Utc>,
    pub outcome: SolverOutcome,
    pub guesses_count: i64,
}

/// Represents a single guess in a solver session
#[derive(Debug, Clone)]
pub struct SolverGuess {
    pub id: i64,
    pub session_id: i64,
    pub guess_number: i64,
    pub word: String,
    pub pool_size_before: i64,
    pub pool_size_after: i64,
    pub entropy: f64,
    pub optimal_word: String,
    pub optimal_entropy: f64,
    pub deviation_score: f64,
}

/// Helper function to serialize feedback to JSON
pub fn serialize_feedback(feedback: &[Feedback]) -> String {
    serde_json::to_string(feedback).unwrap_or_else(|_| "[]".to_string())
}

/// Helper function to deserialize feedback from JSON
pub fn deserialize_feedback(json: &str) -> Vec<Feedback> {
    serde_json::from_str(json).unwrap_or_else(|_| vec![])
}
