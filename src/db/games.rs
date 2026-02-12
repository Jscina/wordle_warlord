use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use sqlx::SqlitePool;

use super::models::{deserialize_feedback, serialize_feedback, Feedback, Game, GameGuess, GameOutcome};

/// Create a new game in the database
pub async fn create_game(
    pool: &SqlitePool,
    timestamp: DateTime<Utc>,
    target_word: String,
) -> Result<i64> {
    let timestamp_str = timestamp.to_rfc3339();
    
    let result = sqlx::query!(
        r#"
        INSERT INTO games (timestamp, target_word, outcome, guesses_count)
        VALUES (?, ?, 'abandoned', 0)
        "#,
        timestamp_str,
        target_word,
    )
    .execute(pool)
    .await
    .context("Failed to create game")?;

    Ok(result.last_insert_rowid())
}

/// Add a guess to a game
pub async fn add_guess(
    pool: &SqlitePool,
    game_id: i64,
    guess_number: i64,
    word: String,
    feedback: Vec<Feedback>,
) -> Result<i64> {
    let feedback_json = serialize_feedback(&feedback);

    let mut tx = pool.begin().await?;

    let result = sqlx::query!(
        r#"
        INSERT INTO game_guesses (game_id, guess_number, word, feedback)
        VALUES (?, ?, ?, ?)
        "#,
        game_id,
        guess_number,
        word,
        feedback_json,
    )
    .execute(&mut *tx)
    .await
    .context("Failed to add guess")?;

    // Update guesses count
    sqlx::query!(
        r#"
        UPDATE games
        SET guesses_count = ?
        WHERE id = ?
        "#,
        guess_number,
        game_id,
    )
    .execute(&mut *tx)
    .await
    .context("Failed to update guesses count")?;

    tx.commit().await?;

    Ok(result.last_insert_rowid())
}

/// Update game outcome
pub async fn update_game_outcome(pool: &SqlitePool, game_id: i64, outcome: GameOutcome) -> Result<()> {
    let outcome_str = outcome.to_string();

    sqlx::query!(
        r#"
        UPDATE games
        SET outcome = ?
        WHERE id = ?
        "#,
        outcome_str,
        game_id,
    )
    .execute(pool)
    .await
    .context("Failed to update game outcome")?;

    Ok(())
}

/// Remove the last guess from a game (for undo functionality)
pub async fn remove_last_guess(pool: &SqlitePool, game_id: i64) -> Result<()> {
    let mut tx = pool.begin().await?;

    // Get the last guess
    let last_guess = sqlx::query!(
        r#"
        SELECT guess_number FROM game_guesses
        WHERE game_id = ?
        ORDER BY guess_number DESC
        LIMIT 1
        "#,
        game_id,
    )
    .fetch_optional(&mut *tx)
    .await?;

    if let Some(guess) = last_guess {
        // Delete the last guess
        sqlx::query!(
            r#"
            DELETE FROM game_guesses
            WHERE game_id = ? AND guess_number = ?
            "#,
            game_id,
            guess.guess_number,
        )
        .execute(&mut *tx)
        .await?;

        // Update guesses count
        let new_count = guess.guess_number - 1;
        sqlx::query!(
            r#"
            UPDATE games
            SET guesses_count = ?
            WHERE id = ?
            "#,
            new_count,
            game_id,
        )
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    Ok(())
}

/// Get a game by ID with all its guesses
pub async fn get_game_with_guesses(pool: &SqlitePool, game_id: i64) -> Result<Option<(Game, Vec<GameGuess>)>> {
    let game_row = sqlx::query!(
        r#"
        SELECT id, timestamp, target_word, outcome, guesses_count
        FROM games
        WHERE id = ?
        "#,
        game_id,
    )
    .fetch_optional(pool)
    .await?;

    let game_row = match game_row {
        Some(row) => row,
        None => return Ok(None),
    };

    let timestamp = DateTime::parse_from_rfc3339(&game_row.timestamp)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now());

    let game = Game {
        id: game_row.id,
        timestamp,
        target_word: game_row.target_word,
        outcome: GameOutcome::from_string(&game_row.outcome).unwrap_or(GameOutcome::Abandoned),
        guesses_count: game_row.guesses_count,
    };

    let guess_rows = sqlx::query!(
        r#"
        SELECT id, game_id, guess_number, word, feedback
        FROM game_guesses
        WHERE game_id = ?
        ORDER BY guess_number ASC
        "#,
        game_id,
    )
    .fetch_all(pool)
    .await?;

    let guesses = guess_rows
        .into_iter()
        .map(|row| GameGuess {
            id: row.id.unwrap_or(0),
            game_id: row.game_id,
            guess_number: row.guess_number,
            word: row.word,
            feedback: deserialize_feedback(&row.feedback),
        })
        .collect();

    Ok(Some((game, guesses)))
}

/// Get all games, ordered by timestamp (most recent first)
pub async fn get_all_games(pool: &SqlitePool) -> Result<Vec<Game>> {
    let rows = sqlx::query!(
        r#"
        SELECT id, timestamp, target_word, outcome, guesses_count
        FROM games
        ORDER BY timestamp DESC
        "#,
    )
    .fetch_all(pool)
    .await?;

    let games = rows
        .into_iter()
        .map(|row| {
            let timestamp = DateTime::parse_from_rfc3339(&row.timestamp)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());
            
            Game {
                id: row.id.unwrap_or(0),
                timestamp,
                target_word: row.target_word,
                outcome: GameOutcome::from_string(&row.outcome).unwrap_or(GameOutcome::Abandoned),
                guesses_count: row.guesses_count,
            }
        })
        .collect();

    Ok(games)
}

/// Get paginated games
pub async fn get_games_paginated(pool: &SqlitePool, limit: i64, offset: i64) -> Result<Vec<Game>> {
    let rows = sqlx::query!(
        r#"
        SELECT id, timestamp, target_word, outcome, guesses_count
        FROM games
        ORDER BY timestamp DESC
        LIMIT ? OFFSET ?
        "#,
        limit,
        offset,
    )
    .fetch_all(pool)
    .await?;

    let games = rows
        .into_iter()
        .map(|row| {
            let timestamp = DateTime::parse_from_rfc3339(&row.timestamp)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());
            
            Game {
                id: row.id.unwrap_or(0),
                timestamp,
                target_word: row.target_word,
                outcome: GameOutcome::from_string(&row.outcome).unwrap_or(GameOutcome::Abandoned),
                guesses_count: row.guesses_count,
            }
        })
        .collect();

    Ok(games)
}

/// Get game statistics
#[derive(Debug)]
pub struct GameStats {
    pub total_games: i64,
    pub wins: i64,
    pub losses: i64,
    pub abandoned: i64,
    pub win_rate: f64,
    pub average_guesses: f64,
    pub guess_distribution: [i64; 6],
}

pub async fn get_game_stats(pool: &SqlitePool) -> Result<GameStats> {
    // Get basic counts
    let counts = sqlx::query!(
        r#"
        SELECT 
            COUNT(*) as total,
            SUM(CASE WHEN outcome = 'won' THEN 1 ELSE 0 END) as "wins!",
            SUM(CASE WHEN outcome = 'lost' THEN 1 ELSE 0 END) as "losses!",
            SUM(CASE WHEN outcome = 'abandoned' THEN 1 ELSE 0 END) as "abandoned!"
        FROM games
        "#,
    )
    .fetch_one(pool)
    .await?;

    let total_games = counts.total;
    let wins = counts.wins;
    let losses = counts.losses;
    let abandoned = counts.abandoned;

    // Calculate win rate (exclude abandoned games)
    let completed_games = wins + losses;
    let win_rate = if completed_games > 0 {
        (wins as f64 / completed_games as f64) * 100.0
    } else {
        0.0
    };

    // Get average guesses for wins only
    let avg_result = sqlx::query!(
        r#"
        SELECT AVG(guesses_count) as avg_guesses
        FROM games
        WHERE outcome = 'won'
        "#,
    )
    .fetch_one(pool)
    .await?;

    let average_guesses = avg_result.avg_guesses.map(|v| v as f64).unwrap_or(0.0);

    // Get guess distribution (1-6 guesses)
    let mut guess_distribution = [0i64; 6];
    for i in 1i64..=6i64 {
        let count = sqlx::query!(
            r#"
            SELECT COUNT(*) as count
            FROM games
            WHERE outcome = 'won' AND guesses_count = ?
            "#,
            i,
        )
        .fetch_one(pool)
        .await?;

        guess_distribution[(i - 1) as usize] = count.count;
    }

    Ok(GameStats {
        total_games,
        wins,
        losses,
        abandoned,
        win_rate,
        average_guesses,
        guess_distribution,
    })
}

/// Get the current game (last game that's not completed)
pub async fn get_current_game(pool: &SqlitePool) -> Result<Option<i64>> {
    let result = sqlx::query!(
        r#"
        SELECT id FROM games
        WHERE outcome = 'abandoned'
        ORDER BY timestamp DESC
        LIMIT 1
        "#,
    )
    .fetch_optional(pool)
    .await?;

    Ok(result.and_then(|r| r.id))
}
