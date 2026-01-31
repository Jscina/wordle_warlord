use anyhow::Result;
use wordle_grep::ui;

use once_cell::sync::OnceCell;
use tracing_appender::rolling;
use tracing_subscriber::prelude::*;
use tracing_subscriber::{EnvFilter, fmt};

static LOG_GUARD: OnceCell<tracing_appender::non_blocking::WorkerGuard> = OnceCell::new();

fn init_logging() {
    let file_appender = rolling::daily("logs", "wordle-grep.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    // Store the guard so it lives for the duration of the program
    LOG_GUARD.set(guard).ok();

    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .with(fmt::layer().with_writer(non_blocking).with_ansi(false))
        .init();
}

fn main() -> Result<()> {
    init_logging();
    ui::run_ui()
}
