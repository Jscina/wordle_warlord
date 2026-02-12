use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::SqlitePool;

use crate::ui::history::{
    GameGuess, GameOutcome, GameRecord,
    solver_types::{SolverGuess, SolverOutcome, SolverSession},
};

/// Load all game records from the database for history display
pub async fn load_game_records(pool: &SqlitePool) -> Result<Vec<GameRecord>> {
    // Get all games ordered by timestamp
    let game_rows = sqlx::query!(
        r#"
        SELECT id, timestamp, target_word, outcome, guesses_count
        FROM games
        ORDER BY timestamp ASC
        "#
    )
    .fetch_all(pool)
    .await?;

    let mut records = Vec::new();

    for game_row in game_rows {
        let timestamp = DateTime::parse_from_rfc3339(&game_row.timestamp)?
            .with_timezone(&Utc);
        
        let outcome = match game_row.outcome.as_str() {
            "won" => GameOutcome::Won {
                guesses: game_row.guesses_count as usize,
            },
            "lost" => GameOutcome::Lost,
            "abandoned" => GameOutcome::Abandoned,
            _ => GameOutcome::Abandoned, // Default fallback
        };

        // Get guesses for this game
        let guess_rows = sqlx::query!(
            r#"
            SELECT word, feedback
            FROM game_guesses
            WHERE game_id = ?
            ORDER BY guess_number ASC
            "#,
            game_row.id
        )
        .fetch_all(pool)
        .await?;

        let guesses: Result<Vec<GameGuess>> = guess_rows
            .iter()
            .map(|row| {
                let db_feedback: Vec<super::models::Feedback> = serde_json::from_str(&row.feedback)?;
                let feedback = db_feedback
                    .into_iter()
                    .map(|f| f.to_solver())
                    .collect();
                
                Ok(GameGuess {
                    word: row.word.clone(),
                    feedback,
                })
            })
            .collect();

        records.push(GameRecord {
            timestamp,
            target_word: game_row.target_word,
            guesses: guesses?,
            outcome,
        });
    }

    Ok(records)
}

/// Load all solver sessions from the database for history display
pub async fn load_solver_sessions(pool: &SqlitePool) -> Result<Vec<SolverSession>> {
    // Get all sessions ordered by timestamp
    let session_rows = sqlx::query!(
        r#"
        SELECT id, timestamp, outcome, guesses_count
        FROM solver_sessions
        ORDER BY timestamp ASC
        "#
    )
    .fetch_all(pool)
    .await?;

    let mut sessions = Vec::new();

    for session_row in session_rows {
        let timestamp = DateTime::parse_from_rfc3339(&session_row.timestamp)?
            .with_timezone(&Utc);
        
        let outcome = match session_row.outcome.as_str() {
            "completed" => SolverOutcome::Completed {
                guesses: session_row.guesses_count as usize,
            },
            "abandoned" => SolverOutcome::Abandoned,
            _ => SolverOutcome::Abandoned, // Default fallback
        };

        // Get guesses for this session
        let guess_rows = sqlx::query!(
            r#"
            SELECT word, pool_size_before, pool_size_after, entropy, 
                   optimal_word, optimal_entropy, deviation_score
            FROM solver_guesses
            WHERE session_id = ?
            ORDER BY guess_number ASC
            "#,
            session_row.id
        )
        .fetch_all(pool)
        .await?;

        let guesses: Vec<SolverGuess> = guess_rows
            .into_iter()
            .map(|row| SolverGuess {
                word: row.word,
                pool_size_before: row.pool_size_before as usize,
                pool_size_after: row.pool_size_after as usize,
                entropy: row.entropy,
                optimal_word: row.optimal_word,
                optimal_entropy: row.optimal_entropy,
                deviation_score: row.deviation_score,
            })
            .collect();

        sessions.push(SolverSession {
            timestamp,
            guesses,
            outcome,
        });
    }

    Ok(sessions)
}
