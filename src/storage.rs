use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::sqlite::SqliteConnectOptions;

#[derive(serde::Serialize, serde::Deserialize)]
struct StoredGuess {
    word: String,
    feedback: String,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct StoredSolverGuess {
    word: String,
    pool_before: usize,
    pool_after: usize,
    entropy: f64,
    optimal_word: String,
    optimal_entropy: f64,
    deviation: f64,
}

pub struct Database {
    pool: sqlx::SqlitePool,
    rt: tokio::runtime::Runtime,
}

impl Database {
    pub fn open(path: &str) -> Result<Self> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;

        let pool = rt.block_on(async {
            let opts = SqliteConnectOptions::new()
                .filename(path)
                .create_if_missing(true);
            sqlx::SqlitePool::connect_with(opts).await
        })?;

        let db = Self { pool, rt };
        db.init_schema()?;
        Ok(db)
    }

    pub fn open_memory() -> Result<Self> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;

        let pool = rt.block_on(async { sqlx::SqlitePool::connect("sqlite::memory:").await })?;

        let db = Self { pool, rt };
        db.init_schema()?;
        Ok(db)
    }

    fn init_schema(&self) -> Result<()> {
        self.rt.block_on(async {
            sqlx::query(
                "CREATE TABLE IF NOT EXISTS games (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    timestamp TEXT NOT NULL,
                    target_word TEXT NOT NULL,
                    outcome TEXT NOT NULL,
                    guess_count INTEGER NOT NULL,
                    guesses_json TEXT NOT NULL
                )",
            )
            .execute(&self.pool)
            .await?;

            sqlx::query(
                "CREATE TABLE IF NOT EXISTS solver_sessions (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    timestamp TEXT NOT NULL,
                    guess_count INTEGER NOT NULL,
                    guesses_json TEXT NOT NULL
                )",
            )
            .execute(&self.pool)
            .await?;

            Ok::<_, anyhow::Error>(())
        })
    }

    pub fn save_game(
        &self,
        timestamp: DateTime<Utc>,
        target_word: &str,
        guesses: &[crate::solver::Guess],
        outcome: &crate::ui::history::GameOutcome,
    ) -> Result<()> {
        let stored: Vec<StoredGuess> = guesses
            .iter()
            .map(|g| {
                let feedback: String = g
                    .feedback
                    .iter()
                    .map(|f| match f {
                        crate::solver::Feedback::Green => 'G',
                        crate::solver::Feedback::Yellow => 'Y',
                        crate::solver::Feedback::Gray => 'X',
                    })
                    .collect();
                StoredGuess {
                    word: g.word.clone(),
                    feedback,
                }
            })
            .collect();

        let guesses_json = serde_json::to_string(&stored)?;
        let outcome_str = match outcome {
            crate::ui::history::GameOutcome::Won { .. } => "won",
            crate::ui::history::GameOutcome::Lost => "lost",
        };
        let timestamp_str = timestamp.to_rfc3339();
        let guess_count = guesses.len() as i64;

        self.rt.block_on(async {
            sqlx::query(
                "INSERT INTO games (timestamp, target_word, outcome, guess_count, guesses_json)
                 VALUES (?, ?, ?, ?, ?)",
            )
            .bind(&timestamp_str)
            .bind(target_word)
            .bind(outcome_str)
            .bind(guess_count)
            .bind(&guesses_json)
            .execute(&self.pool)
            .await?;
            Ok::<_, anyhow::Error>(())
        })
    }

    pub fn load_games(&self) -> Result<Vec<crate::ui::history::GameRecord>> {
        use crate::ui::history::{GameGuess, GameOutcome, GameRecord};

        let rows = self.rt.block_on(async {
            sqlx::query("SELECT timestamp, target_word, outcome, guess_count, guesses_json FROM games ORDER BY timestamp ASC")
                .fetch_all(&self.pool)
                .await
        })?;

        let mut records = Vec::new();
        for row in rows {
            use sqlx::Row;
            let timestamp_str: String = row.get("timestamp");
            let target_word: String = row.get("target_word");
            let outcome_str: String = row.get("outcome");
            let guess_count: i64 = row.get("guess_count");
            let guesses_json: String = row.get("guesses_json");

            let timestamp = DateTime::parse_from_rfc3339(&timestamp_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());

            let outcome = match outcome_str.as_str() {
                "won" => GameOutcome::Won {
                    guesses: guess_count as usize,
                },
                _ => GameOutcome::Lost,
            };

            let stored_guesses: Vec<StoredGuess> =
                serde_json::from_str(&guesses_json).unwrap_or_default();

            let guesses: Vec<GameGuess> = stored_guesses
                .into_iter()
                .map(|sg| {
                    let feedback: Vec<crate::solver::Feedback> = sg
                        .feedback
                        .chars()
                        .map(|c| {
                            crate::solver::Feedback::try_from(c)
                                .unwrap_or(crate::solver::Feedback::Gray)
                        })
                        .collect();
                    GameGuess {
                        word: sg.word,
                        feedback,
                    }
                })
                .collect();

            records.push(GameRecord {
                timestamp,
                target_word,
                guesses,
                outcome,
            });
        }

        Ok(records)
    }

    pub fn save_solver_session(
        &self,
        timestamp: DateTime<Utc>,
        guesses: &[crate::ui::history::solver_types::SolverGuess],
    ) -> Result<()> {
        let stored: Vec<StoredSolverGuess> = guesses
            .iter()
            .map(|g| StoredSolverGuess {
                word: g.word.clone(),
                pool_before: g.pool_size_before,
                pool_after: g.pool_size_after,
                entropy: g.entropy,
                optimal_word: g.optimal_word.clone(),
                optimal_entropy: g.optimal_entropy,
                deviation: g.deviation_score,
            })
            .collect();

        let guesses_json = serde_json::to_string(&stored)?;
        let timestamp_str = timestamp.to_rfc3339();
        let guess_count = guesses.len() as i64;

        self.rt.block_on(async {
            sqlx::query(
                "INSERT INTO solver_sessions (timestamp, guess_count, guesses_json)
                 VALUES (?, ?, ?)",
            )
            .bind(&timestamp_str)
            .bind(guess_count)
            .bind(&guesses_json)
            .execute(&self.pool)
            .await?;
            Ok::<_, anyhow::Error>(())
        })
    }

    pub fn load_solver_sessions(
        &self,
    ) -> Result<Vec<crate::ui::history::solver_types::SolverSession>> {
        use crate::ui::history::solver_types::{SolverGuess, SolverOutcome, SolverSession};

        let rows = self.rt.block_on(async {
            sqlx::query(
                "SELECT timestamp, guess_count, guesses_json FROM solver_sessions ORDER BY timestamp ASC",
            )
            .fetch_all(&self.pool)
            .await
        })?;

        let mut sessions = Vec::new();
        for row in rows {
            use sqlx::Row;
            let timestamp_str: String = row.get("timestamp");
            let guess_count: i64 = row.get("guess_count");
            let guesses_json: String = row.get("guesses_json");

            let timestamp = DateTime::parse_from_rfc3339(&timestamp_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());

            let stored_guesses: Vec<StoredSolverGuess> =
                serde_json::from_str(&guesses_json).unwrap_or_default();

            let guesses: Vec<SolverGuess> = stored_guesses
                .into_iter()
                .map(|sg| SolverGuess {
                    word: sg.word,
                    pool_size_before: sg.pool_before,
                    pool_size_after: sg.pool_after,
                    entropy: sg.entropy,
                    optimal_word: sg.optimal_word,
                    optimal_entropy: sg.optimal_entropy,
                    deviation_score: sg.deviation,
                })
                .collect();

            sessions.push(SolverSession {
                timestamp,
                guesses,
                outcome: SolverOutcome::Completed {
                    guesses: guess_count as usize,
                },
            });
        }

        Ok(sessions)
    }
}
