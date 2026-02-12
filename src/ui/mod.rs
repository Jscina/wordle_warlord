mod app;
mod handlers;
pub mod history;
mod rendering;
#[cfg(test)]
mod tests;
mod types;

pub use app::App;
pub use types::{GameMode, InputStatus, LogBuffer, ParsedInput};

use anyhow::Result;
use crossterm::{
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use sqlx::SqlitePool;
use std::io::stdout;

use crate::wordlist::{load_solutions, load_words};

/// Entry point for running the UI.
pub async fn run_ui(db_pool: SqlitePool) -> Result<()> {
    let words = load_words()?;
    let solution_words = load_solutions()?;
    let logs = LogBuffer::new();

    let mut app = App::new(words, solution_words, 5, logs.clone(), db_pool);

    let mut stdout = stdout();
    enable_raw_mode()?;
    execute!(stdout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = app.run(&mut terminal);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}
