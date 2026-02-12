mod parser;
pub mod solver_types;
pub mod types;

pub use parser::{parse_game_history, parse_solver_history};
pub use solver_types::{SolverGuess, SolverOutcome, SolverSession, SolverStats};
pub use types::{GameGuess, GameOutcome, GameRecord, HistoryData, HistoryStats, HistoryViewMode};
