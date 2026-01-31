use crate::solver::{Feedback, SolverState};
use std::collections::{HashMap, HashSet};

#[derive(Debug)]
pub struct LetterAnalysis {
    pub frequencies: HashMap<char, usize>,
    pub total_words: usize,
    pub max_frequency: usize,
}

#[derive(Debug)]
pub struct PositionAnalysis {
    pub possible_letters: Vec<Vec<char>>,
    pub solved_positions: Vec<Option<char>>,
    pub position_frequencies: Vec<HashMap<char, usize>>,
}

#[derive(Debug)]
pub struct ConstraintSummary {
    pub greens: Vec<(char, usize, String)>,
    pub yellows: Vec<(char, Vec<usize>, String)>,
    pub grays: Vec<char>,
    pub min_counts: HashMap<char, usize>,
    pub max_counts: HashMap<char, usize>,
}

#[derive(Debug)]
pub struct SolutionPoolStats {
    pub total_remaining: usize,
    pub eliminated_percentage: f64,
    pub entropy: f64,
}

pub fn compute_letter_analysis(words: &[&String]) -> LetterAnalysis {
    let mut frequencies = HashMap::new();

    for word in words {
        let mut seen = HashSet::new();
        for c in word.chars() {
            if seen.insert(c) {
                *frequencies.entry(c).or_insert(0) += 1;
            }
        }
    }

    let max_frequency = frequencies.values().copied().max().unwrap_or(0);

    LetterAnalysis {
        frequencies,
        total_words: words.len(),
        max_frequency,
    }
}

pub fn compute_position_analysis(words: &[&String], solver: &SolverState) -> PositionAnalysis {
    let word_len = solver.word_len();

    let mut possible_letters = vec![Vec::new(); word_len];
    let mut position_frequencies = vec![HashMap::new(); word_len];
    let mut solved_positions = vec![None; word_len];

    for word in words {
        for (pos, c) in word.chars().enumerate() {
            if !possible_letters[pos].contains(&c) {
                possible_letters[pos].push(c);
            }

            *position_frequencies[pos].entry(c).or_insert(0) += 1;
        }
    }

    for pos in 0..word_len {
        if possible_letters[pos].len() == 1 {
            solved_positions[pos] = possible_letters[pos].first().copied();
        }

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
    let mut gray_letters = HashSet::new();

    let mut min_counts = HashMap::new();
    let mut max_counts = HashMap::new();

    for guess in solver.guesses() {
        let chars: Vec<char> = guess.word.chars().collect();

        let mut gy_counts = HashMap::new();
        let mut total_counts = HashMap::new();

        for &c in &chars {
            *total_counts.entry(c).or_insert(0) += 1;
        }

        for (pos, (&c, &fb)) in chars.iter().zip(&guess.feedback).enumerate() {
            match fb {
                Feedback::Green => {
                    greens.push((c, pos, guess.word.clone()));
                    *gy_counts.entry(c).or_insert(0) += 1;
                }
                Feedback::Yellow => {
                    if let Some((_, positions, _)) = yellows.iter_mut().find(|(l, _, _)| *l == c) {
                        positions.push(pos);
                    } else {
                        yellows.push((c, vec![pos], guess.word.clone()));
                    }

                    *gy_counts.entry(c).or_insert(0) += 1;
                }
                Feedback::Gray => {}
            }
        }

        // Apply min counts
        for (c, count) in &gy_counts {
            let entry = min_counts.entry(*c).or_insert(0);
            *entry = (*entry).max(*count);
        }

        // Determine gray constraints
        for (&c, &guess_total) in &total_counts {
            let gy = *gy_counts.get(&c).unwrap_or(&0);

            if guess_total > gy {
                gray_letters.insert(c);
                max_counts
                    .entry(c)
                    .and_modify(|m: &mut usize| *m = (*m).min(gy))
                    .or_insert(gy);
            }
        }
    }

    ConstraintSummary {
        greens,
        yellows,
        grays: gray_letters.into_iter().collect(),
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
        (1.0 - total_remaining as f64 / all_words.len() as f64) * 100.0
    };

    let entropy = if total_remaining <= 1 {
        0.0
    } else {
        let mut letter_counts = HashMap::new();

        for word in filtered {
            let mut seen = HashSet::new();
            for c in word.chars() {
                if seen.insert(c) {
                    *letter_counts.entry(c).or_insert(0) += 1;
                }
            }
        }

        letter_counts
            .values()
            .map(|&count| {
                let p = count as f64 / total_remaining as f64;
                -p * p.log2()
            })
            .sum()
    };

    SolutionPoolStats {
        total_remaining,
        eliminated_percentage,
        entropy,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solver::{Feedback, Guess, SolverState};

    fn make_solver_state(guesses: Vec<(&str, Vec<Feedback>)>, word_len: usize) -> SolverState {
        let mut solver = SolverState::new(word_len);
        for (word, feedback) in guesses {
            solver.add_guess(Guess::new(word.to_string(), feedback));
        }
        solver
    }

    #[test]
    fn test_compute_letter_analysis() {
        let apple = "apple".to_string();
        let angle = "angle".to_string();
        let ample = "ample".to_string();

        let words = vec![&apple, &angle, &ample];

        let analysis = compute_letter_analysis(words.as_slice());
        assert_eq!(analysis.total_words, 3);
        assert_eq!(analysis.frequencies.get(&'a'), Some(&3));
        assert_eq!(analysis.frequencies.get(&'p'), Some(&2));
        assert_eq!(analysis.max_frequency, 3);
    }

    #[test]
    fn test_compute_position_analysis() {
        let apple = "apple".to_string();
        let angle = "angle".to_string();
        let ample = "ample".to_string();

        let words = vec![&apple, &angle, &ample];

        let solver = SolverState::new(5);
        let analysis = compute_position_analysis(words.as_slice(), &solver);
        assert_eq!(analysis.possible_letters[0], vec!['a']);
        assert!(analysis.possible_letters[1].contains(&'p'));
        assert!(analysis.position_frequencies[4].contains_key(&'e'));
    }

    #[test]
    fn test_compute_constraint_summary() {
        let guesses = vec![
            (
                "apple",
                vec![
                    Feedback::Green,
                    Feedback::Gray,
                    Feedback::Gray,
                    Feedback::Gray,
                    Feedback::Gray,
                ],
            ),
            (
                "angle",
                vec![
                    Feedback::Green,
                    Feedback::Yellow,
                    Feedback::Gray,
                    Feedback::Gray,
                    Feedback::Gray,
                ],
            ),
        ];
        let solver = make_solver_state(guesses, 5);
        let summary = compute_constraint_summary(&solver);
        assert!(
            summary
                .greens
                .iter()
                .any(|(c, pos, _)| *c == 'a' && *pos == 0)
        );
        assert!(summary.yellows.iter().any(|(c, _, _)| *c == 'n'));
        assert!(summary.grays.contains(&'p'));
    }

    #[test]
    fn test_compute_solution_pool_stats() {
        let all_words = vec![
            "apple".to_string(),
            "angle".to_string(),
            "ample".to_string(),
        ];
        let apple = "apple".to_string();
        let filtered: Vec<&String> = vec![&apple];
        let stats = compute_solution_pool_stats(all_words.as_slice(), &filtered);
        assert_eq!(stats.total_remaining, 1);
        assert!(stats.eliminated_percentage > 0.0);
        assert_eq!(stats.entropy, 0.0);
    }
}
