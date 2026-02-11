use std::sync::{Arc, Mutex};

use crate::solver::Feedback;

pub const MAX_LOG_LINES: usize = 300;

/// Thread-safe circular log buffer with a maximum capacity.
#[derive(Clone)]
pub struct LogBuffer {
    inner: Arc<Mutex<Vec<String>>>,
}

impl LogBuffer {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn push(&self, msg: String) {
        let mut buf = self.inner.lock().unwrap();
        buf.push(msg);
        if buf.len() > MAX_LOG_LINES {
            buf.remove(0);
        }
    }

    pub fn lines(&self) -> Vec<String> {
        self.inner.lock().unwrap().clone()
    }
}

impl Default for LogBuffer {
    fn default() -> Self {
        Self::new()
    }
}

/// Input validation status.
pub enum InputStatus {
    Incomplete,
    Invalid(&'static str),
    Valid,
}

/// Result of parsing user input.
pub enum ParsedInput {
    Incomplete,
    Invalid,
    Valid {
        word: String,
        feedback: Vec<Feedback>,
    },
}

/// Application operating mode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GameMode {
    Solver,
    Game,
    History,
}
