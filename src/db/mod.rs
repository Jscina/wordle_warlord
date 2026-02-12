pub mod models;
pub mod games;
pub mod solver;
pub mod history;

use anyhow::{Context, Result};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use std::path::PathBuf;
use std::str::FromStr;

/// Get the path to the database file using platform-specific data directory
pub fn get_db_path() -> Result<PathBuf> {
    let mut path = dirs::data_dir()
        .context("Unable to determine data directory for your platform")?;
    
    path.push("wordle-warlord");
    
    // Create directory if it doesn't exist
    std::fs::create_dir_all(&path)
        .context("Failed to create wordle-warlord data directory")?;
    
    path.push("history.db");
    Ok(path)
}

/// Create a connection pool to the SQLite database
pub async fn create_pool() -> Result<SqlitePool> {
    let db_path = get_db_path()?;
    
    let options = SqliteConnectOptions::from_str(&format!("sqlite:{}", db_path.display()))?
        .create_if_missing(true)
        .foreign_keys(true);
    
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await
        .context("Failed to connect to database")?;
    
    // Run migrations
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .context("Failed to run database migrations")?;
    
    Ok(pool)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_pool() {
        let pool = create_pool().await;
        assert!(pool.is_ok());
    }
}
