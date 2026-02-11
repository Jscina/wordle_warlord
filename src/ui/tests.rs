//! UI module tests.

use super::{
    app::App,
    handlers::{GameHandler, HistoryHandler, InputHandler, SolverHandler},
    history::{HistoryData, HistoryViewMode},
    types::{GameMode, InputStatus, LogBuffer},
};
use crate::solver::{Feedback, Guess};

/// Helper function to create a test app with a minimal word list.
fn create_test_app() -> App {
    let words = vec![
        "raise".to_string(),
        "stone".to_string(),
        "slate".to_string(),
        "crane".to_string(),
        "house".to_string(),
        "apple".to_string(),
        "world".to_string(),
        "magic".to_string(),
    ];
    let solution_words = words.clone();
    let logs = LogBuffer::new();

    App::new(words, solution_words, 5, logs)
}

#[cfg(test)]
mod app_tests {
    use super::*;

    #[test]
    fn test_app_initialization() {
        let app = create_test_app();

        assert_eq!(app.mode, GameMode::Solver);
        assert!(app.input.is_empty());
        assert_eq!(app.remaining_guesses, 6);
        assert!(!app.game_won);
        assert!(!app.game_over);
        assert!(app.show_suggestions);
        assert!(app.show_analysis);
        assert!(app.target_word.is_none());
    }

    #[test]
    fn test_log_buffer() {
        let logs = LogBuffer::new();

        logs.push("Test message 1".to_string());
        logs.push("Test message 2".to_string());

        let lines = logs.lines();
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0], "Test message 1");
        assert_eq!(lines[1], "Test message 2");
    }

    #[test]
    fn test_log_buffer_max_capacity() {
        let logs = LogBuffer::new();

        // Push more than MAX_LOG_LINES
        for i in 0..350 {
            logs.push(format!("Message {}", i));
        }

        let lines = logs.lines();
        assert!(lines.len() <= super::super::types::MAX_LOG_LINES);
    }
}

#[cfg(test)]
mod input_handler_tests {
    use super::*;

    #[test]
    fn test_input_validation_solver_mode_incomplete() {
        let mut app = create_test_app();
        app.mode = GameMode::Solver;
        app.input = "raise".to_string();

        let handler = InputHandler::new(&mut app);
        let status = handler.input_status();

        assert!(matches!(status, InputStatus::Incomplete));
    }

    #[test]
    fn test_input_validation_solver_mode_valid() {
        let mut app = create_test_app();
        app.mode = GameMode::Solver;
        app.input = "raise GYXXX".to_string();

        let handler = InputHandler::new(&mut app);
        let status = handler.input_status();

        assert!(matches!(status, InputStatus::Valid));
    }

    #[test]
    fn test_input_validation_solver_mode_invalid_pattern() {
        let mut app = create_test_app();
        app.mode = GameMode::Solver;
        app.input = "raise GZXXX".to_string(); // Z is invalid

        let handler = InputHandler::new(&mut app);
        let status = handler.input_status();

        assert!(matches!(status, InputStatus::Invalid(_)));
    }

    #[test]
    fn test_input_validation_solver_mode_wrong_length() {
        let mut app = create_test_app();
        app.mode = GameMode::Solver;
        app.input = "raise GYX".to_string(); // Pattern too short

        let handler = InputHandler::new(&mut app);
        let status = handler.input_status();

        assert!(matches!(status, InputStatus::Invalid(_)));
    }

    #[test]
    fn test_input_validation_solver_mode_invalid_word() {
        let mut app = create_test_app();
        app.mode = GameMode::Solver;
        app.input = "zzzzz GGGGG".to_string(); // Word not in list

        let handler = InputHandler::new(&mut app);
        let status = handler.input_status();

        assert!(matches!(status, InputStatus::Invalid(_)));
    }

    #[test]
    fn test_input_validation_game_mode_valid() {
        let mut app = create_test_app();
        app.mode = GameMode::Game;
        app.target_word = Some("stone".to_string());
        app.input = "raise".to_string();

        let handler = InputHandler::new(&mut app);
        let status = handler.input_status();

        assert!(matches!(status, InputStatus::Valid));
    }

    #[test]
    fn test_input_validation_game_mode_incomplete() {
        let mut app = create_test_app();
        app.mode = GameMode::Game;
        app.target_word = Some("stone".to_string());
        app.input = "rai".to_string();

        let handler = InputHandler::new(&mut app);
        let status = handler.input_status();

        // Word too short is invalid, not incomplete
        assert!(matches!(status, InputStatus::Invalid(_)));
    }

    #[test]
    fn test_input_validation_game_mode_invalid_word() {
        let mut app = create_test_app();
        app.mode = GameMode::Game;
        app.target_word = Some("stone".to_string());
        app.input = "zzzzz".to_string();

        let handler = InputHandler::new(&mut app);
        let status = handler.input_status();

        assert!(matches!(status, InputStatus::Invalid(_)));
    }

    #[test]
    fn test_input_validation_game_mode_empty() {
        let mut app = create_test_app();
        app.mode = GameMode::Game;
        app.target_word = Some("stone".to_string());
        app.input = "".to_string();

        let handler = InputHandler::new(&mut app);
        let status = handler.input_status();

        assert!(matches!(status, InputStatus::Incomplete));
    }
}

#[cfg(test)]
mod game_handler_tests {
    use super::*;

    #[test]
    fn test_start_new_game() {
        let mut app = create_test_app();

        GameHandler::new(&mut app).start_new_game();

        assert_eq!(app.mode, GameMode::Game);
        assert!(app.target_word.is_some());
        assert_eq!(app.remaining_guesses, 6);
        assert!(!app.game_won);
        assert!(!app.game_over);
        assert!(!app.show_suggestions);
        assert!(!app.show_analysis);
        assert!(app.input.is_empty());
        assert_eq!(app.solver.guesses().len(), 0);
    }

    #[test]
    fn test_toggle_game_mode_from_solver() {
        let mut app = create_test_app();
        app.mode = GameMode::Solver;

        GameHandler::new(&mut app).toggle_game_mode();

        assert_eq!(app.mode, GameMode::Game);
        assert!(app.target_word.is_some());
    }

    #[test]
    fn test_toggle_game_mode_from_game() {
        let mut app = create_test_app();
        GameHandler::new(&mut app).start_new_game();

        GameHandler::new(&mut app).toggle_game_mode();

        assert_eq!(app.mode, GameMode::Solver);
    }

    #[test]
    fn test_check_game_state_won() {
        let mut app = create_test_app();
        app.mode = GameMode::Game;
        app.target_word = Some("stone".to_string());

        let all_green = vec![
            Feedback::Green,
            Feedback::Green,
            Feedback::Green,
            Feedback::Green,
            Feedback::Green,
        ];

        GameHandler::new(&mut app).check_game_state(&all_green);

        assert!(app.game_won);
        assert!(app.game_over);
    }

    #[test]
    fn test_check_game_state_not_won() {
        let mut app = create_test_app();
        app.mode = GameMode::Game;
        app.target_word = Some("stone".to_string());
        app.remaining_guesses = 3;

        let mixed = vec![
            Feedback::Green,
            Feedback::Yellow,
            Feedback::Gray,
            Feedback::Green,
            Feedback::Gray,
        ];

        GameHandler::new(&mut app).check_game_state(&mixed);

        assert!(!app.game_won);
        assert!(!app.game_over);
    }

    #[test]
    fn test_check_game_state_out_of_guesses() {
        let mut app = create_test_app();
        app.mode = GameMode::Game;
        app.target_word = Some("stone".to_string());
        app.remaining_guesses = 0;

        let mixed = vec![
            Feedback::Green,
            Feedback::Yellow,
            Feedback::Gray,
            Feedback::Green,
            Feedback::Gray,
        ];

        GameHandler::new(&mut app).check_game_state(&mixed);

        assert!(!app.game_won);
        assert!(app.game_over);
    }

    #[test]
    fn test_game_state_resets_on_new_game() {
        let mut app = create_test_app();

        // Start first game
        GameHandler::new(&mut app).start_new_game();
        app.remaining_guesses = 2;
        app.game_over = true;
        app.game_won = true;

        // Start new game
        GameHandler::new(&mut app).start_new_game();

        assert_eq!(app.remaining_guesses, 6);
        assert!(!app.game_over);
        assert!(!app.game_won);
        assert_eq!(app.solver.guesses().len(), 0);
    }
}

#[cfg(test)]
mod solver_handler_tests {
    use super::*;

    #[test]
    fn test_undo_guess() {
        let mut app = create_test_app();

        // Add a guess
        let guess = Guess::new(
            "raise".to_string(),
            vec![
                Feedback::Gray,
                Feedback::Yellow,
                Feedback::Gray,
                Feedback::Gray,
                Feedback::Green,
            ],
        );
        app.solver.add_guess(guess);

        assert_eq!(app.solver.guesses().len(), 1);

        // Undo the guess
        SolverHandler::new(&mut app).undo_guess();

        assert_eq!(app.solver.guesses().len(), 0);
    }

    #[test]
    fn test_undo_guess_empty() {
        let mut app = create_test_app();

        assert_eq!(app.solver.guesses().len(), 0);

        // Undo with no guesses should not crash
        SolverHandler::new(&mut app).undo_guess();

        assert_eq!(app.solver.guesses().len(), 0);
    }

    #[test]
    fn test_recompute_updates_suggestions() {
        let mut app = create_test_app();

        // Add a guess first - recompute only generates suggestions when guesses exist
        let guess = Guess::new(
            "raise".to_string(),
            vec![
                Feedback::Gray,
                Feedback::Yellow,
                Feedback::Gray,
                Feedback::Gray,
                Feedback::Green,
            ],
        );
        app.solver.add_guess(guess);

        // Initially no suggestions computed
        app.suggestions.clear();

        // Recompute should populate suggestions
        SolverHandler::new(&mut app).recompute();

        assert!(!app.suggestions.is_empty());
    }

    #[test]
    fn test_recompute_with_guess_narrows_suggestions() {
        let mut app = create_test_app();

        // Get initial suggestion count
        SolverHandler::new(&mut app).recompute();
        let initial_count = app.suggestions.len();

        // Add a guess that filters words
        let guess = Guess::new(
            "raise".to_string(),
            vec![
                Feedback::Gray,
                Feedback::Gray,
                Feedback::Gray,
                Feedback::Gray,
                Feedback::Green,
            ],
        );
        app.solver.add_guess(guess);

        // Recompute with constraint
        SolverHandler::new(&mut app).recompute();
        let filtered_count = app.suggestions.len();

        // Should have fewer suggestions after constraint
        assert!(filtered_count <= initial_count);
    }
}

#[cfg(test)]
mod history_handler_tests {
    use super::*;
    use crate::ui::history::{GameOutcome, GameRecord};
    use chrono::Utc;

    fn create_test_history_data() -> HistoryData {
        let games = vec![
            GameRecord {
                timestamp: Utc::now(),
                target_word: "stone".to_string(),
                guesses: vec![],
                outcome: GameOutcome::Won { guesses: 3 },
            },
            GameRecord {
                timestamp: Utc::now(),
                target_word: "raise".to_string(),
                guesses: vec![],
                outcome: GameOutcome::Lost,
            },
        ];
        HistoryData::new(games, Vec::new())
    }

    #[test]
    fn test_enter_history_mode() {
        let mut app = create_test_app();
        app.mode = GameMode::Solver;

        HistoryHandler::new(&mut app).enter_history_mode();

        assert_eq!(app.mode, GameMode::History);
        assert_eq!(app.history_view_mode, HistoryViewMode::Stats);
        assert_eq!(app.history_page, 0);
    }

    #[test]
    fn test_exit_history_mode() {
        let mut app = create_test_app();
        app.mode = GameMode::History;

        HistoryHandler::new(&mut app).exit_history_mode();

        assert_eq!(app.mode, GameMode::Solver);
    }

    #[test]
    fn test_cycle_view_mode_stats_to_list() {
        let mut app = create_test_app();
        app.history_view_mode = HistoryViewMode::Stats;
        app.history_data = Some(create_test_history_data());

        HistoryHandler::new(&mut app).cycle_view_mode();

        assert_eq!(app.history_view_mode, HistoryViewMode::List);
    }

    #[test]
    fn test_cycle_view_mode_list_to_stats_no_selection() {
        let mut app = create_test_app();
        app.history_view_mode = HistoryViewMode::List;
        app.history_data = Some(create_test_history_data());

        HistoryHandler::new(&mut app).cycle_view_mode();

        // With the new Solver view, List cycles to Solver when no game is selected
        assert_eq!(app.history_view_mode, HistoryViewMode::Solver);
    }

    #[test]
    fn test_cycle_view_mode_list_to_detail_with_selection() {
        let mut app = create_test_app();
        app.history_view_mode = HistoryViewMode::List;
        let mut data = create_test_history_data();
        data.select_game(0);
        app.history_data = Some(data);

        HistoryHandler::new(&mut app).cycle_view_mode();

        assert_eq!(app.history_view_mode, HistoryViewMode::Detail);
    }

    #[test]
    fn test_cycle_view_mode_detail_to_stats() {
        let mut app = create_test_app();
        app.history_view_mode = HistoryViewMode::Detail;
        app.history_data = Some(create_test_history_data());

        HistoryHandler::new(&mut app).cycle_view_mode();

        assert_eq!(app.history_view_mode, HistoryViewMode::Stats);
    }

    #[test]
    fn test_select_game_on_page() {
        let mut app = create_test_app();
        app.history_data = Some(create_test_history_data());
        app.history_page = 0;

        HistoryHandler::new(&mut app).select_game_on_page(0);

        assert_eq!(app.history_view_mode, HistoryViewMode::Detail);
        assert!(app.history_data.as_ref().unwrap().selected_game().is_some());
    }

    #[test]
    fn test_select_game_invalid_index() {
        let mut app = create_test_app();
        app.history_data = Some(create_test_history_data());
        app.history_page = 0;

        // Try to select index that doesn't exist
        HistoryHandler::new(&mut app).select_game_on_page(99);

        // Should not crash, and no game should be selected
        assert!(app.history_data.as_ref().unwrap().selected_game().is_none());
    }

    #[test]
    fn test_pagination() {
        let mut app = create_test_app();
        app.history_data = Some(create_test_history_data());
        app.history_page = 0;

        // Go to next page
        HistoryHandler::new(&mut app).next_page();

        // With only 2 games, should stay on page 0
        assert_eq!(app.history_page, 0);

        // Go back
        HistoryHandler::new(&mut app).prev_page();
        assert_eq!(app.history_page, 0);
    }

    #[test]
    fn test_return_to_list() {
        let mut app = create_test_app();
        let mut data = create_test_history_data();
        data.select_game(0);
        app.history_data = Some(data);
        app.history_view_mode = HistoryViewMode::Detail;

        HistoryHandler::new(&mut app).return_to_list();

        assert_eq!(app.history_view_mode, HistoryViewMode::List);
        assert!(app.history_data.as_ref().unwrap().selected_game().is_none());
    }

    #[test]
    fn test_return_to_stats() {
        let mut app = create_test_app();
        let mut data = create_test_history_data();
        data.select_game(0);
        app.history_data = Some(data);
        app.history_view_mode = HistoryViewMode::Detail;

        HistoryHandler::new(&mut app).return_to_stats();

        assert_eq!(app.history_view_mode, HistoryViewMode::Stats);
        assert!(app.history_data.as_ref().unwrap().selected_game().is_none());
    }
}

#[cfg(test)]
mod mode_switching_tests {
    use super::*;

    #[test]
    fn test_solver_to_game_transition() {
        let mut app = create_test_app();
        app.mode = GameMode::Solver;
        app.show_analysis = true;

        GameHandler::new(&mut app).toggle_game_mode();

        assert_eq!(app.mode, GameMode::Game);
        assert!(!app.show_analysis); // Should be hidden in game mode
        assert!(!app.show_suggestions); // Should be hidden in game mode
    }

    #[test]
    fn test_game_to_solver_transition() {
        let mut app = create_test_app();
        GameHandler::new(&mut app).start_new_game();
        app.show_analysis = false;

        GameHandler::new(&mut app).toggle_game_mode();

        assert_eq!(app.mode, GameMode::Solver);
        // Analysis visibility should remain as set (solver mode respects toggle)
    }

    #[test]
    fn test_solver_to_history_transition() {
        let mut app = create_test_app();
        app.mode = GameMode::Solver;

        HistoryHandler::new(&mut app).enter_history_mode();

        assert_eq!(app.mode, GameMode::History);
        assert_eq!(app.history_view_mode, HistoryViewMode::Stats);
    }

    #[test]
    fn test_history_to_solver_transition() {
        let mut app = create_test_app();
        app.mode = GameMode::History;

        HistoryHandler::new(&mut app).exit_history_mode();

        assert_eq!(app.mode, GameMode::Solver);
    }

    #[test]
    fn test_game_state_persists_when_switching_modes() {
        let mut app = create_test_app();

        // Add a guess in solver mode
        let guess = Guess::new(
            "raise".to_string(),
            vec![
                Feedback::Gray,
                Feedback::Yellow,
                Feedback::Gray,
                Feedback::Gray,
                Feedback::Green,
            ],
        );
        app.solver.add_guess(guess);
        let guess_count = app.solver.guesses().len();

        // Switch to history and back
        HistoryHandler::new(&mut app).enter_history_mode();
        HistoryHandler::new(&mut app).exit_history_mode();

        // Solver state should be preserved
        assert_eq!(app.solver.guesses().len(), guess_count);
    }
}

#[cfg(test)]
mod analysis_toggle_tests {
    use super::*;

    #[test]
    fn test_analysis_shown_by_default_in_solver() {
        let app = create_test_app();

        assert_eq!(app.mode, GameMode::Solver);
        assert!(app.show_analysis);
    }

    #[test]
    fn test_analysis_hidden_by_default_in_game() {
        let mut app = create_test_app();

        GameHandler::new(&mut app).start_new_game();

        assert_eq!(app.mode, GameMode::Game);
        assert!(!app.show_analysis);
    }

    #[test]
    fn test_analysis_toggle_in_game_mode() {
        let mut app = create_test_app();
        GameHandler::new(&mut app).start_new_game();

        assert!(!app.show_analysis);

        // Toggle on
        app.show_analysis = !app.show_analysis;
        assert!(app.show_analysis);

        // Toggle off
        app.show_analysis = !app.show_analysis;
        assert!(!app.show_analysis);
    }

    #[test]
    fn test_suggestions_toggle_in_game_mode() {
        let mut app = create_test_app();
        GameHandler::new(&mut app).start_new_game();

        assert!(!app.show_suggestions);

        // Toggle on
        app.show_suggestions = !app.show_suggestions;
        assert!(app.show_suggestions);

        // Toggle off
        app.show_suggestions = !app.show_suggestions;
        assert!(!app.show_suggestions);
    }
}
