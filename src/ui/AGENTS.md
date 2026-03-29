# UI MODULE

TUI layer built on Ratatui + Crossterm. State machine with 3 modes (Solver/Game/History), handler-per-concern pattern, and modular rendering.

## STRUCTURE

```
ui/
├── mod.rs              # Bootstrap: run_ui() — terminal init, word loading, event loop
├── app.rs              # App struct (central state), run() loop, draw() dispatch
├── types.rs            # GameMode enum, InputStatus, ParsedInput, LogBuffer
├── tests.rs            # 705 lines, 40+ tests across 7 suites
├── handlers/
│   ├── input_handler.rs  # Key event router (375 lines) — dispatches by mode + modifier
│   ├── game_handler.rs   # Game lifecycle: start, check state, toggle mode
│   ├── solver_handler.rs # Undo, recompute suggestions + analysis
│   └── history_handler.rs # View cycling, pagination, game selection
├── rendering/
│   ├── mod.rs            # draw() — layout construction, panel dispatch
│   ├── guesses.rs        # Colored guess history (Green/Yellow/Gray tiles)
│   ├── suggestions.rs    # Ranked word suggestions list
│   ├── input_field.rs    # Input bar with validation coloring
│   ├── status.rs         # Mode indicator / game status bar
│   ├── logs.rs           # Log panel
│   ├── analysis/         # 4 analysis panels: letters, positions, constraints, pool
│   └── history/          # 4 history views: stats, list, detail, solver
└── history/
    ├── parser.rs         # Log file parser (529 lines) — regex over tracing output
    ├── types.rs          # GameRecord, HistoryData, HistoryStats, GameOutcome
    └── solver_types.rs   # SolverSession, SolverStats, SolverOutcome
```

## WHERE TO LOOK

| Task | File | Notes |
|------|------|-------|
| Add keyboard shortcut | `handlers/input_handler.rs` | Match on `KeyCode` + modifiers, delegate to handler |
| New game mode | `types.rs` (add variant) → `app.rs` → `input_handler.rs` → `rendering/mod.rs` |
| New analysis panel | `rendering/analysis/` + register in `rendering/mod.rs` `draw()` layout |
| New history view | `history/types.rs` (add `HistoryViewMode` variant) → `rendering/history/` → `handlers/history_handler.rs` |
| Fix game logic | `handlers/game_handler.rs` | `check_game_state()`, `start_new_game()` |
| Change layout | `rendering/mod.rs` | Ratatui `Layout::default().constraints([...])` |
| Parse new log event | `history/parser.rs` | Regex-based extraction from tracing log lines |

## CONVENTIONS

- **Handler pattern**: Struct borrows `&mut App`, constructed inline per use. No persistent handler state.
  ```rust
  SolverHandler::new(&mut self).recompute_analysis();  // in app.rs run loop
  InputHandler::new(self).handle_key(key);              // returns bool (quit?)
  ```
- **Rendering**: `draw_*` methods implemented on `App` in separate files via `impl App` blocks. Each panel is a standalone method receiving `Frame` + `Rect`.
- **Analysis recomputation**: Lazy — `analysis_dirty` flag set on guess add/undo, recomputed at top of run loop before draw.
- **Visibility**: All App fields are `pub(in crate::ui)` — handlers and rendering access them directly, but nothing outside ui/ can.
- **History view cycling**: `Stats → List → Detail (if selected) / Solver (if not) → Stats`

## ANTI-PATTERNS

- Do NOT add persistent state to handlers — they are ephemeral `&mut App` wrappers
- Do NOT access App fields from outside `crate::ui` — use the public API (`run_ui()`, exported types)
- Do NOT skip `analysis_dirty = true` when modifying solver state — panels will show stale data
- History parser depends on exact tracing log format — changes to `app.log()` messages will break parsing

## NOTES

- **Input validation differs by mode**: Solver expects `<word> <pattern>`, Game expects just `<word>`
- **Game mode toggles**: `show_suggestions` and `show_analysis` default OFF in Game, always ON in Solver
- **Solver sessions**: Tracked via `solver_session_active/start/paused` fields for history stats
- **Test helper**: `create_test_app()` uses 8-word vocabulary — sufficient for unit tests, not representative of real word counts
