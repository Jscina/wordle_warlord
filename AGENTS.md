# PROJECT KNOWLEDGE BASE

**Generated:** 2026-03-29
**Commit:** 9931171
**Branch:** main

## OVERVIEW

Rust TUI Wordle solver + local game + history viewer. Ratatui/Crossterm frontend, constraint-based solver engine, log-parsed game history. Hybrid binary (`wordle-warlord`) + library (`wordle_warlord`) crate.

## STRUCTURE

```
./
├── src/
│   ├── main.rs           # Binary entry: logging init → ui::run_ui()
│   ├── lib.rs             # Library root: exports analysis, scoring, solver, ui, wordlist
│   ├── solver.rs          # Constraint matching engine (Green/Yellow/Gray feedback)
│   ├── analysis.rs        # Letter frequency, position analysis, constraint summaries, entropy
│   ├── scoring.rs         # Word ranking by unique letter frequency + solution bonus
│   ├── wordlist.rs        # Downloads + caches wordlists from GitHub on first run
│   └── ui/                # See src/ui/AGENTS.md
├── assets/                # Screenshot PNGs for README
├── .github/workflows/     # CI (test/lint/build), audit (weekly), release (multi-platform)
└── logs/                  # Runtime logs (daily rolling, gitignored)
```

## WHERE TO LOOK

| Task | Location | Notes |
|------|----------|-------|
| Solver logic | `src/solver.rs` | `matches()` = core constraint filter, `generate_feedback()` = game mode |
| Add analysis metric | `src/analysis.rs` | 4 compute functions, each returns a typed struct |
| Change word scoring | `src/scoring.rs` | `score_and_sort()` — unique letter freq + `SOLUTION_BONUS` (10) |
| Wordlist sources | `src/wordlist.rs` | URLs hardcoded, files cached as `words.txt`/`solutions.txt` in CWD |
| UI changes | `src/ui/` | App state in `app.rs`, handlers in `handlers/`, rendering in `rendering/` |
| Add keyboard shortcut | `src/ui/handlers/input_handler.rs` | Central key dispatch, delegates to mode-specific handlers |
| History parsing | `src/ui/history/parser.rs` | Parses structured tracing logs — 529 lines, largest file |
| CI pipeline | `.github/workflows/ci.yml` | test → lint → build (parallel jobs) |
| Release process | `.github/workflows/release.yml` | Tag `v*` → 5-platform matrix build → GitHub Release |

## CODE MAP

| Symbol | Type | Location | Role |
|--------|------|----------|------|
| `App` | struct | `src/ui/app.rs` | Central state container (25+ fields), owns SolverState |
| `SolverState` | struct | `src/solver.rs` | Tracks guesses, filters candidates via `filter()` |
| `Feedback` | enum | `src/solver.rs` | Green/Yellow/Gray — `TryFrom<char>` for parsing |
| `GameMode` | enum | `src/ui/types.rs` | Solver / Game / History |
| `InputHandler` | struct | `src/ui/handlers/input_handler.rs` | Central key event dispatcher |
| `GameHandler` | struct | `src/ui/handlers/game_handler.rs` | Game mode logic (start, check state, toggle) |
| `SolverHandler` | struct | `src/ui/handlers/solver_handler.rs` | Undo, recompute suggestions/analysis |
| `HistoryHandler` | struct | `src/ui/handlers/history_handler.rs` | History navigation, pagination, view cycling |
| `HistoryData` | struct | `src/ui/history/types.rs` | Parsed game records + solver sessions |
| `LogBuffer` | struct | `src/ui/types.rs` | Thread-safe circular buffer (Arc<Mutex<Vec>>, max 300) |
| `run_ui()` | fn | `src/ui/mod.rs` | Bootstrap: load words → init terminal → App::run() |
| `matches()` | fn | `src/solver.rs` | Core: does candidate match guess+feedback? 3-pass algorithm |
| `score_and_sort()` | fn | `src/scoring.rs` | Rank words by unique letter frequency + solution bonus |

## CONVENTIONS

- **Edition 2024** — uses let-chains (`if let Some(c) = x && *c > 0`)
- **Visibility**: `pub(in crate::ui)` on App fields — accessible within ui module tree only
- **Handler pattern**: Each handler borrows `&mut App`, constructed per-use: `SolverHandler::new(&mut app).undo_guess()`
- **Dual logging**: `app.log()` writes to both `tracing::info!` (file) and `LogBuffer` (UI)
- **No build.rs** — pure Cargo build, no codegen
- **Blocking HTTP**: `reqwest::blocking` for wordlist download (acceptable — runs once at startup)

## ANTI-PATTERNS (THIS PROJECT)

- `panic!("Invalid feedback")` in test helper `feedback_vec()` — test-only, not production path
- `unwrap()` on Mutex lock in `LogBuffer` — acceptable for single-threaded TUI, but would poison on panic
- Wordlists cached in CWD (`words.txt`, `solutions.txt`) — not XDG-compliant, but intentional simplicity
- History parsed from log files — fragile if log format changes, but avoids database dependency

## COMMANDS

```bash
cargo run                              # Debug run (starts in Solver mode)
cargo build --release                  # Release binary → target/release/wordle-warlord
cargo test --all-features --verbose    # 65 tests across 5 modules
cargo fmt --all -- --check             # Format check (CI-enforced)
cargo clippy --all-features -- -D warnings  # Lint (warnings = errors in CI)
cargo audit --deny warnings            # Security audit (weekly CI)
```

## NOTES

- **Solver returns empty results** = user entered wrong feedback, not a bug
- **First run downloads wordlists** via HTTP — needs network access
- **Log rotation**: daily rolling to `logs/wordle-warlord.log.*` — `OnceCell` guard keeps appender alive
- **3-pass constraint matching** in `matches()`: greens (exact + count reduction) → yellows (present elsewhere) → grays (no remaining count)
- **Release**: bump version in `Cargo.toml` → `git tag v{version}` → push tag → CI builds 5 platforms
