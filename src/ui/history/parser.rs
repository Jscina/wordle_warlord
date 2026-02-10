//! Log file parser for extracting game history.

use std::fs;
use std::path::Path;

use chrono::DateTime;

use crate::solver::generate_feedback;

use super::types::{GameGuess, GameOutcome, GameRecord};

/// Parse all log files in the logs directory and extract game records.
pub fn parse_game_history(logs_dir: &str) -> Result<Vec<GameRecord>, String> {
    let logs_path = Path::new(logs_dir);

    if !logs_path.exists() {
        return Ok(Vec::new());
    }

    // Read all log files matching the pattern
    let mut log_files = Vec::new();

    match fs::read_dir(logs_path) {
        Ok(entries) => {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(filename) = path.file_name() {
                    if let Some(name) = filename.to_str() {
                        if name.starts_with("wordle-warlord.log") {
                            log_files.push(path);
                        }
                    }
                }
            }
        }
        Err(e) => return Err(format!("Failed to read logs directory: {}", e)),
    }

    // Sort log files by name (which sorts by date due to naming convention)
    log_files.sort();

    // Parse all log files
    let mut all_games = Vec::new();

    for log_file in log_files {
        match parse_log_file(&log_file) {
            Ok(mut games) => all_games.append(&mut games),
            Err(e) => {
                // Log the error but continue processing other files
                eprintln!("Warning: Failed to parse {:?}: {}", log_file, e);
            }
        }
    }

    // Sort games by timestamp (oldest first)
    all_games.sort_by_key(|g| g.timestamp);

    Ok(all_games)
}

/// Parse a single log file and extract game records.
fn parse_log_file(path: &Path) -> Result<Vec<GameRecord>, String> {
    let content = fs::read_to_string(path).map_err(|e| format!("Failed to read file: {}", e))?;

    let mut games = Vec::new();
    let mut current_game: Option<(DateTime<chrono::Utc>, String, Vec<String>)> = None;
    let mut current_mode = "Solver"; // Track current mode (start in Solver mode)

    for line in content.lines() {
        // Parse timestamp
        let timestamp = parse_timestamp(line);

        // Track mode switches
        if line.contains("Switching to game mode") || line.contains("Starting new game") {
            current_mode = "Game";
        } else if line.contains("Switching to solver mode") {
            current_mode = "Solver";
        }

        // Check for game start
        if let Some(target) = extract_new_game(line) {
            // If there's an existing game, mark it as abandoned
            if let Some((ts, target_word, guesses)) = current_game.take() {
                let game = build_game_record(ts, target_word, guesses, GameOutcome::Abandoned);
                games.push(game);
            }

            // Start a new game
            if let Some(ts) = timestamp {
                current_game = Some((ts, target, Vec::new()));
                current_mode = "Game"; // New game always puts us in Game mode
            }
            continue;
        }

        // Check for guess submission (only in Game mode)
        if current_mode == "Game" {
            if let Some(word) = extract_game_guess(line) {
                if let Some((_, _, ref mut guesses)) = current_game {
                    guesses.push(word);
                }
                continue;
            }
        }

        // Check for undo request (only in Game mode, and only if we have a current game)
        if current_mode == "Game" && line.contains("Undo requested") {
            if let Some((_, _, ref mut guesses)) = current_game {
                // Remove the last guess if any exist
                guesses.pop();
            }
            continue;
        }

        // Check for game won
        if line.contains("Game won!") {
            if let Some((ts, target_word, guesses)) = current_game.take() {
                let outcome = GameOutcome::Won {
                    guesses: guesses.len(),
                };
                let game = build_game_record(ts, target_word, guesses, outcome);
                games.push(game);
            }
            continue;
        }

        // Check for game lost
        if line.contains("Game over: out of guesses") {
            if let Some((ts, target_word, guesses)) = current_game.take() {
                let game = build_game_record(ts, target_word, guesses, GameOutcome::Lost);
                games.push(game);
            }
            continue;
        }
    }

    // If there's still a game in progress at end of file, mark as abandoned
    if let Some((ts, target_word, guesses)) = current_game {
        let game = build_game_record(ts, target_word, guesses, GameOutcome::Abandoned);
        games.push(game);
    }

    Ok(games)
}

/// Parse timestamp from log line.
fn parse_timestamp(line: &str) -> Option<DateTime<chrono::Utc>> {
    // Log format: "2026-02-10T15:20:49.305070Z  INFO ..."
    let parts: Vec<&str> = line.split_whitespace().collect();
    if let Some(ts_str) = parts.first() {
        DateTime::parse_from_rfc3339(ts_str)
            .ok()
            .map(|dt| dt.with_timezone(&chrono::Utc))
    } else {
        None
    }
}

/// Extract target word from "New game started" log line.
fn extract_new_game(line: &str) -> Option<String> {
    if let Some(pos) = line.find("New game started with target word: ") {
        let target = &line[pos + 35..];
        Some(target.trim().to_string())
    } else {
        None
    }
}

/// Extract guess word from "Game guess submitted" log line.
fn extract_game_guess(line: &str) -> Option<String> {
    if let Some(pos) = line.find("Game guess submitted: ") {
        let word = &line[pos + 22..];
        Some(word.trim().to_string())
    } else {
        None
    }
}

/// Build a GameRecord from parsed data, generating feedback for each guess.
fn build_game_record(
    timestamp: DateTime<chrono::Utc>,
    target_word: String,
    guess_words: Vec<String>,
    outcome: GameOutcome,
) -> GameRecord {
    let guesses = guess_words
        .into_iter()
        .map(|word| {
            let feedback = generate_feedback(&target_word, &word);
            GameGuess { word, feedback }
        })
        .collect();

    GameRecord {
        timestamp,
        target_word,
        guesses,
        outcome,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_new_game() {
        let line = "2026-02-10T15:20:49.305070Z  INFO wordle_warlord::ui: New game started with target word: savvy";
        assert_eq!(extract_new_game(line), Some("savvy".to_string()));
    }

    #[test]
    fn test_extract_game_guess() {
        let line =
            "2026-02-10T15:20:51.898425Z  INFO wordle_warlord::ui: Game guess submitted: raise";
        assert_eq!(extract_game_guess(line), Some("raise".to_string()));
    }

    #[test]
    fn test_parse_timestamp() {
        let line = "2026-02-10T15:20:49.305070Z  INFO wordle_warlord::ui: Test";
        let ts = parse_timestamp(line);
        assert!(ts.is_some());
    }

    #[test]
    fn test_undo_handling() {
        use std::io::Write;
        use tempfile::NamedTempFile;

        // Create a temporary log file with undo events
        let mut file = NamedTempFile::new().unwrap();
        writeln!(
            file,
            "2026-02-10T15:20:49.305070Z  INFO wordle_warlord::ui: New game started with target word: savvy"
        )
        .unwrap();
        writeln!(
            file,
            "2026-02-10T15:20:51.898425Z  INFO wordle_warlord::ui: Game guess submitted: raise"
        )
        .unwrap();
        writeln!(
            file,
            "2026-02-10T15:20:54.181349Z  INFO wordle_warlord::ui: Game guess submitted: salty"
        )
        .unwrap();
        writeln!(
            file,
            "2026-02-10T15:20:55.000000Z  INFO wordle_warlord::ui: Undo requested"
        )
        .unwrap();
        writeln!(
            file,
            "2026-02-10T15:20:56.924491Z  INFO wordle_warlord::ui: Game guess submitted: savvy"
        )
        .unwrap();
        writeln!(
            file,
            "2026-02-10T15:20:56.924710Z  INFO wordle_warlord::ui: Game won!"
        )
        .unwrap();
        file.flush().unwrap();

        let games = parse_log_file(file.path()).unwrap();

        assert_eq!(games.len(), 1);
        let game = &games[0];
        assert_eq!(game.target_word, "savvy");
        assert_eq!(game.guesses.len(), 2); // Should be 2, not 3 (salty was undone)
        assert_eq!(game.guesses[0].word, "raise");
        assert_eq!(game.guesses[1].word, "savvy");
        assert!(matches!(game.outcome, GameOutcome::Won { .. }));
    }

    #[test]
    fn test_multiple_undos() {
        use std::io::Write;
        use tempfile::NamedTempFile;

        // Create a temporary log file with multiple undo events
        let mut file = NamedTempFile::new().unwrap();
        writeln!(
            file,
            "2026-02-10T15:20:49.305070Z  INFO wordle_warlord::ui: New game started with target word: crane"
        )
        .unwrap();
        writeln!(
            file,
            "2026-02-10T15:20:51.000000Z  INFO wordle_warlord::ui: Game guess submitted: raise"
        )
        .unwrap();
        writeln!(
            file,
            "2026-02-10T15:20:52.000000Z  INFO wordle_warlord::ui: Game guess submitted: salty"
        )
        .unwrap();
        writeln!(
            file,
            "2026-02-10T15:20:53.000000Z  INFO wordle_warlord::ui: Game guess submitted: brain"
        )
        .unwrap();
        writeln!(
            file,
            "2026-02-10T15:20:54.000000Z  INFO wordle_warlord::ui: Undo requested"
        )
        .unwrap();
        writeln!(
            file,
            "2026-02-10T15:20:55.000000Z  INFO wordle_warlord::ui: Undo requested"
        )
        .unwrap();
        writeln!(
            file,
            "2026-02-10T15:20:56.000000Z  INFO wordle_warlord::ui: Game guess submitted: crane"
        )
        .unwrap();
        writeln!(
            file,
            "2026-02-10T15:20:57.000000Z  INFO wordle_warlord::ui: Game won!"
        )
        .unwrap();
        file.flush().unwrap();

        let games = parse_log_file(file.path()).unwrap();

        assert_eq!(games.len(), 1);
        let game = &games[0];
        assert_eq!(game.guesses.len(), 2); // Should be 2 (raise, crane)
        assert_eq!(game.guesses[0].word, "raise");
        assert_eq!(game.guesses[1].word, "crane");
    }
}
