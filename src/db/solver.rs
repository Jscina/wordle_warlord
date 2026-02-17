use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use sqlx::SqlitePool;

use super::models::{SolverGuess, SolverOutcome, SolverSession};

/// Parameters for adding a solver guess
pub struct SolverGuessParams {
    pub guess_number: i64,
    pub word: String,
    pub pool_size_before: i64,
    pub pool_size_after: i64,
    pub entropy: f64,
    pub optimal_word: String,
    pub optimal_entropy: f64,
    pub deviation_score: f64,
}

impl SolverGuessParams {
    /// Create a new SolverGuessParams instance
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        guess_number: i64,
        word: String,
        pool_size_before: i64,
        pool_size_after: i64,
        entropy: f64,
        optimal_word: String,
        optimal_entropy: f64,
        deviation_score: f64,
    ) -> Self {
        Self {
            guess_number,
            word,
            pool_size_before,
            pool_size_after,
            entropy,
            optimal_word,
            optimal_entropy,
            deviation_score,
        }
    }
}

/// Create a new solver session in the database
pub async fn create_session(pool: &SqlitePool, timestamp: DateTime<Utc>) -> Result<i64> {
    let timestamp_str = timestamp.to_rfc3339();

    let result = sqlx::query!(
        r#"
        INSERT INTO solver_sessions (timestamp, outcome, guesses_count)
        VALUES (?, 'abandoned', 0)
        "#,
        timestamp_str,
    )
    .execute(pool)
    .await
    .context("Failed to create solver session")?;

    Ok(result.last_insert_rowid())
}

/// Add a guess to a solver session
pub async fn add_guess(
    pool: &SqlitePool,
    session_id: i64,
    params: SolverGuessParams,
) -> Result<i64> {
    let mut tx = pool.begin().await?;

    let result = sqlx::query!(
        r#"
        INSERT INTO solver_guesses (
            session_id, guess_number, word, pool_size_before, pool_size_after,
            entropy, optimal_word, optimal_entropy, deviation_score
        )
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
        session_id,
        params.guess_number,
        params.word,
        params.pool_size_before,
        params.pool_size_after,
        params.entropy,
        params.optimal_word,
        params.optimal_entropy,
        params.deviation_score,
    )
    .execute(&mut *tx)
    .await
    .context("Failed to add solver guess")?;

    // Update guesses count
    sqlx::query!(
        r#"
        UPDATE solver_sessions
        SET guesses_count = ?
        WHERE id = ?
        "#,
        params.guess_number,
        session_id,
    )
    .execute(&mut *tx)
    .await
    .context("Failed to update guesses count")?;

    tx.commit().await?;

    Ok(result.last_insert_rowid())
}

/// Update solver session outcome
pub async fn update_session_outcome(
    pool: &SqlitePool,
    session_id: i64,
    outcome: SolverOutcome,
) -> Result<()> {
    let outcome_str = outcome.to_string();

    sqlx::query!(
        r#"
        UPDATE solver_sessions
        SET outcome = ?
        WHERE id = ?
        "#,
        outcome_str,
        session_id,
    )
    .execute(pool)
    .await
    .context("Failed to update solver session outcome")?;

    Ok(())
}

/// Remove the last guess from a solver session (for undo functionality)
pub async fn remove_last_guess(pool: &SqlitePool, session_id: i64) -> Result<()> {
    let mut tx = pool.begin().await?;

    // Get the last guess
    let last_guess = sqlx::query!(
        r#"
        SELECT guess_number FROM solver_guesses
        WHERE session_id = ?
        ORDER BY guess_number DESC
        LIMIT 1
        "#,
        session_id,
    )
    .fetch_optional(&mut *tx)
    .await?;

    if let Some(guess) = last_guess {
        // Delete the last guess
        sqlx::query!(
            r#"
            DELETE FROM solver_guesses
            WHERE session_id = ? AND guess_number = ?
            "#,
            session_id,
            guess.guess_number,
        )
        .execute(&mut *tx)
        .await?;

        // Update guesses count
        let new_count = guess.guess_number - 1;
        sqlx::query!(
            r#"
            UPDATE solver_sessions
            SET guesses_count = ?
            WHERE id = ?
            "#,
            new_count,
            session_id,
        )
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    Ok(())
}

/// Get a solver session by ID with all its guesses
pub async fn get_session_with_guesses(
    pool: &SqlitePool,
    session_id: i64,
) -> Result<Option<(SolverSession, Vec<SolverGuess>)>> {
    let session_row = sqlx::query!(
        r#"
        SELECT id, timestamp, outcome, guesses_count
        FROM solver_sessions
        WHERE id = ?
        "#,
        session_id,
    )
    .fetch_optional(pool)
    .await?;

    let session_row = match session_row {
        Some(row) => row,
        None => return Ok(None),
    };

    let timestamp = DateTime::parse_from_rfc3339(&session_row.timestamp)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now());

    let session = SolverSession {
        id: session_row.id,
        timestamp,
        outcome: SolverOutcome::from_string(&session_row.outcome)
            .unwrap_or(SolverOutcome::Abandoned),
        guesses_count: session_row.guesses_count,
    };

    let guess_rows = sqlx::query!(
        r#"
        SELECT id, session_id, guess_number, word, pool_size_before, pool_size_after,
               entropy, optimal_word, optimal_entropy, deviation_score
        FROM solver_guesses
        WHERE session_id = ?
        ORDER BY guess_number ASC
        "#,
        session_id,
    )
    .fetch_all(pool)
    .await?;

    let guesses = guess_rows
        .into_iter()
        .map(|row| SolverGuess {
            id: row.id.unwrap_or(0),
            session_id: row.session_id,
            guess_number: row.guess_number,
            word: row.word,
            pool_size_before: row.pool_size_before,
            pool_size_after: row.pool_size_after,
            entropy: row.entropy,
            optimal_word: row.optimal_word,
            optimal_entropy: row.optimal_entropy,
            deviation_score: row.deviation_score,
        })
        .collect();

    Ok(Some((session, guesses)))
}

/// Get all solver sessions, ordered by timestamp (most recent first)
pub async fn get_all_sessions(pool: &SqlitePool) -> Result<Vec<SolverSession>> {
    let rows = sqlx::query!(
        r#"
        SELECT id, timestamp, outcome, guesses_count
        FROM solver_sessions
        ORDER BY timestamp DESC
        "#,
    )
    .fetch_all(pool)
    .await?;

    let sessions = rows
        .into_iter()
        .map(|row| {
            let timestamp = DateTime::parse_from_rfc3339(&row.timestamp)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());

            SolverSession {
                id: row.id.unwrap_or(0),
                timestamp,
                outcome: SolverOutcome::from_string(&row.outcome)
                    .unwrap_or(SolverOutcome::Abandoned),
                guesses_count: row.guesses_count,
            }
        })
        .collect();

    Ok(sessions)
}

/// Get paginated solver sessions
pub async fn get_sessions_paginated(
    pool: &SqlitePool,
    limit: i64,
    offset: i64,
) -> Result<Vec<SolverSession>> {
    let rows = sqlx::query!(
        r#"
        SELECT id, timestamp, outcome, guesses_count
        FROM solver_sessions
        ORDER BY timestamp DESC
        LIMIT ? OFFSET ?
        "#,
        limit,
        offset,
    )
    .fetch_all(pool)
    .await?;

    let sessions = rows
        .into_iter()
        .map(|row| {
            let timestamp = DateTime::parse_from_rfc3339(&row.timestamp)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());

            SolverSession {
                id: row.id.unwrap_or(0),
                timestamp,
                outcome: SolverOutcome::from_string(&row.outcome)
                    .unwrap_or(SolverOutcome::Abandoned),
                guesses_count: row.guesses_count,
            }
        })
        .collect();

    Ok(sessions)
}

/// Get solver session statistics
#[derive(Debug)]
pub struct SolverStats {
    pub total_sessions: i64,
    pub completed_sessions: i64,
    pub abandoned_sessions: i64,
    pub average_guesses: f64,
    pub average_entropy: f64,
    pub optimal_adherence: f64,
    pub average_deviation: f64,
}

pub async fn get_solver_stats(pool: &SqlitePool) -> Result<SolverStats> {
    // Get basic counts
    let counts = sqlx::query!(
        r#"
        SELECT 
            COUNT(*) as total,
            SUM(CASE WHEN outcome = 'completed' THEN 1 ELSE 0 END) as "completed!",
            SUM(CASE WHEN outcome = 'abandoned' THEN 1 ELSE 0 END) as "abandoned!"
        FROM solver_sessions
        "#,
    )
    .fetch_one(pool)
    .await?;

    let total_sessions = counts.total;
    let completed_sessions = counts.completed;
    let abandoned_sessions = counts.abandoned;

    // Get average guesses for completed sessions
    let avg_guesses_result = sqlx::query!(
        r#"
        SELECT AVG(guesses_count) as avg_guesses
        FROM solver_sessions
        WHERE outcome = 'completed'
        "#,
    )
    .fetch_one(pool)
    .await?;

    let average_guesses = avg_guesses_result
        .avg_guesses
        .map(|v| v as f64)
        .unwrap_or(0.0);

    // Get average entropy across all guesses
    let avg_entropy_result = sqlx::query!(
        r#"
        SELECT AVG(entropy) as avg_entropy
        FROM solver_guesses
        "#,
    )
    .fetch_one(pool)
    .await?;

    let average_entropy = avg_entropy_result.avg_entropy.unwrap_or(0.0);

    // Calculate optimal adherence (percentage of guesses that were optimal)
    let optimal_count = sqlx::query!(
        r#"
        SELECT COUNT(*) as "count!"
        FROM solver_guesses
        WHERE deviation_score >= 0
        "#,
    )
    .fetch_one(pool)
    .await?;

    let total_guesses = sqlx::query!(
        r#"
        SELECT COUNT(*) as count
        FROM solver_guesses
        "#,
    )
    .fetch_one(pool)
    .await?;

    let optimal_adherence = if total_guesses.count > 0 {
        (optimal_count.count as f64 / total_guesses.count as f64) * 100.0
    } else {
        0.0
    };

    // Get average deviation score
    let avg_deviation_result = sqlx::query!(
        r#"
        SELECT AVG(deviation_score) as avg_deviation
        FROM solver_guesses
        "#,
    )
    .fetch_one(pool)
    .await?;

    let average_deviation = avg_deviation_result.avg_deviation.unwrap_or(0.0);

    Ok(SolverStats {
        total_sessions,
        completed_sessions,
        abandoned_sessions,
        average_guesses,
        average_entropy,
        optimal_adherence,
        average_deviation,
    })
}

/// Get the current solver session (last session that's not completed)
pub async fn get_current_session(pool: &SqlitePool) -> Result<Option<i64>> {
    let result = sqlx::query!(
        r#"
        SELECT id FROM solver_sessions
        WHERE outcome = 'abandoned'
        ORDER BY timestamp DESC
        LIMIT 1
        "#,
    )
    .fetch_optional(pool)
    .await?;

    Ok(result.map(|r| r.id))
}
