-- Create games table
CREATE TABLE games (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp TEXT NOT NULL,
    target_word TEXT NOT NULL,
    outcome TEXT NOT NULL CHECK (outcome IN ('won', 'lost', 'abandoned')),
    guesses_count INTEGER NOT NULL,
    created_at TEXT DEFAULT (datetime('now'))
);

-- Create game guesses table (one-to-many with games)
CREATE TABLE game_guesses (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    game_id INTEGER NOT NULL,
    guess_number INTEGER NOT NULL,
    word TEXT NOT NULL,
    feedback TEXT NOT NULL,
    FOREIGN KEY (game_id) REFERENCES games(id) ON DELETE CASCADE
);

-- Create solver sessions table
CREATE TABLE solver_sessions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp TEXT NOT NULL,
    outcome TEXT NOT NULL CHECK (outcome IN ('completed', 'abandoned')),
    guesses_count INTEGER NOT NULL,
    created_at TEXT DEFAULT (datetime('now'))
);

-- Create solver guesses table (one-to-many with solver_sessions)
CREATE TABLE solver_guesses (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id INTEGER NOT NULL,
    guess_number INTEGER NOT NULL,
    word TEXT NOT NULL,
    pool_size_before INTEGER NOT NULL,
    pool_size_after INTEGER NOT NULL,
    entropy REAL NOT NULL,
    optimal_word TEXT NOT NULL,
    optimal_entropy REAL NOT NULL,
    deviation_score REAL NOT NULL,
    FOREIGN KEY (session_id) REFERENCES solver_sessions(id) ON DELETE CASCADE
);

-- Create indexes for common queries
CREATE INDEX idx_games_timestamp ON games(timestamp DESC);
CREATE INDEX idx_games_outcome ON games(outcome);
CREATE INDEX idx_solver_sessions_timestamp ON solver_sessions(timestamp DESC);
CREATE INDEX idx_game_guesses_game_id ON game_guesses(game_id);
CREATE INDEX idx_solver_guesses_session_id ON solver_guesses(session_id);
