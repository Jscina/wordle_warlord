//! Game history tracking and display.
//!
//! This module provides functionality for parsing game logs and displaying
//! historical game data, statistics, and performance analytics.

mod parser;
mod types;

pub use parser::parse_game_history;
pub use types::{GameOutcome, GameRecord, HistoryData, HistoryStats, HistoryViewMode};
