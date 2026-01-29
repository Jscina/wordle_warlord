use crate::solver::{Feedback, SolverState};
use std::collections::HashMap;

pub struct LetterAnalysis {
    pub frequencies: HashMap<char, usize>,
    pub total_words: usize,
    pub max_frequency: usize,
}

pub struct PositionAnalysis {
    pub possible_letters: Vec<Vec<char>>,
    pub solved_positions: Vec<Option<char>>,
    pub position_frequencies: Vec<HashMap<char, usize>>,
}

pub struct ConstraintSummary {
    pub greens: Vec<(char, usize, String)>, // (letter, position, guess_word)
    pub yellows: Vec<(char, Vec<usize>, String)>, // (letter, positions, guess_word)
    pub grays: Vec<char>,
    pub min_counts: HashMap<char, usize>,
    pub max_counts: HashMap<char, usize>,
}

pub struct SolutionPoolStats {
    pub total_remaining: usize,
    pub eliminated_percentage: f64,
    pub entropy: f64,
}

pub fn compute_letter_analysis(words: &[&String]) -> LetterAnalysis {
    let mut frequencies: HashMap<char, usize> = HashMap::new();
    let mut total_words = 0;

    for word in words {
        total_words += 1;
        let mut seen: HashMap<char, bool> = HashMap::new();

        for c in word.chars() {
            if !seen.get(&c).unwrap_or(&false) {
                *frequencies.entry(c).or_insert(0) += 1;
                seen.insert(c, true);
            }
        }
    }

    let max_frequency = frequencies.values().max().copied().unwrap_or(0);

    LetterAnalysis {
        frequencies,
        total_words,
        max_frequency,
    }
}

pub fn compute_position_analysis(words: &[&String], solver: &SolverState) -> PositionAnalysis {
    let word_len = solver.word_len();
    let mut possible_letters: Vec<Vec<char>> = vec![Vec::new(); word_len];
    let mut position_frequencies: Vec<HashMap<char, usize>> = vec![HashMap::new(); word_len];
    let mut solved_positions: Vec<Option<char>> = vec![None; word_len];

    // Collect all possible letters for each position
    for word in words {
        for (pos, c) in word.chars().enumerate() {
            if !possible_letters[pos].contains(&c) {
                possible_letters[pos].push(c);
            }
            *position_frequencies[pos].entry(c).or_insert(0) += 1;
        }
    }

    // Check for solved positions (all words have same letter)
    for pos in 0..word_len {
        if let Some(&first_letter) = possible_letters[pos].first() {
            let all_same = possible_letters[pos].iter().all(|&c| c == first_letter);
            if all_same {
                solved_positions[pos] = Some(first_letter);
            }
        }
    }

    // Sort letters by frequency (descending) within each position
    for pos in 0..word_len {
        possible_letters[pos].sort_by(|a, b| {
            position_frequencies[pos]
                .get(b)
                .unwrap_or(&0)
                .cmp(position_frequencies[pos].get(a).unwrap_or(&0))
        });
    }

    PositionAnalysis {
        possible_letters,
        solved_positions,
        position_frequencies,
    }
}

pub fn compute_constraint_summary(solver: &SolverState) -> ConstraintSummary {
    let mut greens = Vec::new();
    let mut yellows: Vec<(char, Vec<usize>, String)> = Vec::new();
    let mut grays = Vec::new();
    let mut min_counts: HashMap<char, usize> = HashMap::new();
    let mut max_counts: HashMap<char, usize> = HashMap::new();

    for guess in solver.guesses() {
        let word_chars: Vec<char> = guess.word.chars().collect();
        let mut guess_letter_counts: HashMap<char, usize> = HashMap::new();

        // First pass: track green and yellow constraints
        for (pos, (&c, &fb)) in word_chars.iter().zip(guess.feedback.iter()).enumerate() {
            match fb {
                Feedback::Green => {
                    greens.push((c, pos, guess.word.clone()));
                    *min_counts.entry(c).or_insert(0) += 1;
                    *guess_letter_counts.entry(c).or_insert(0) += 1;
                }
                Feedback::Yellow => {
                    // Find existing yellow entry or create new one
                    if let Some((_, positions, _)) =
                        yellows.iter_mut().find(|(letter, _, _)| *letter == c)
                    {
                        positions.push(pos);
                    } else {
                        yellows.push((c, vec![pos], guess.word.clone()));
                    }
                    *min_counts.entry(c).or_insert(0) += 1;
                    *guess_letter_counts.entry(c).or_insert(0) += 1;
                }
                Feedback::Gray => {}
            }
        }

        // Second pass: track gray constraints with proper counting
        for &c in &word_chars {
            let green_yellow_count = min_counts.get(&c).unwrap_or(&0);
            let guess_count = guess_letter_counts.get(&c).unwrap_or(&0);

            // If letter appears more in guess than justified by green/yellow, gray constraints apply
            if guess_count > green_yellow_count {
                grays.push(c);
                max_counts.insert(c, *green_yellow_count);
            }
        }
    }

    // Remove duplicates from grays
    grays.sort();
    grays.dedup();

    ConstraintSummary {
        greens,
        yellows,
        grays,
        min_counts,
        max_counts,
    }
}

pub fn compute_solution_pool_stats(
    all_words: &[String],
    filtered: &[&String],
) -> SolutionPoolStats {
    let total_remaining = filtered.len();
    let eliminated_percentage = if all_words.is_empty() {
        0.0
    } else {
        (1.0 - (total_remaining as f64 / all_words.len() as f64)) * 100.0
    };

    let entropy = if total_remaining <= 1 {
        0.0
    } else {
        let _log2_n = (total_remaining as f64).log2();
        let mut entropy_sum = 0.0;

        // Calculate entropy based on letter frequencies
        let mut letter_counts: HashMap<char, usize> = HashMap::new();
        for word in filtered {
            let mut seen: HashMap<char, bool> = HashMap::new();
            for c in word.chars() {
                if !seen.get(&c).unwrap_or(&false) {
                    *letter_counts.entry(c).or_insert(0) += 1;
                    seen.insert(c, true);
                }
            }
        }

        for &count in letter_counts.values() {
            let probability = count as f64 / total_remaining as f64;
            if probability > 0.0 {
                entropy_sum -= probability * probability.log2();
            }
        }

        entropy_sum
    };

    SolutionPoolStats {
        total_remaining,
        eliminated_percentage,
        entropy,
    }
}
