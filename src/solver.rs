use anyhow::Result;
use std::{collections::HashMap, convert::TryFrom};

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Feedback {
    Green,
    Yellow,
    Gray,
}

impl TryFrom<char> for Feedback {
    type Error = char;

    fn try_from(value: char) -> Result<Self, Self::Error> {
        match value.to_ascii_uppercase() {
            'G' => Ok(Feedback::Green),
            'Y' => Ok(Feedback::Yellow),
            'X' => Ok(Feedback::Gray),
            _ => Err(value),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Guess {
    pub word: String,
    pub feedback: Vec<Feedback>,
}

#[derive(Debug)]
pub struct SolverState {
    word_len: usize,
    guesses: Vec<Guess>,
}

impl SolverState {
    pub fn new(word_len: usize) -> Self {
        Self {
            word_len,
            guesses: Vec::new(),
        }
    }

    pub fn guesses(&self) -> &[Guess] {
        &self.guesses
    }

    pub fn add_guess(&mut self, word: String, feedback: Vec<Feedback>) {
        assert_eq!(word.len(), self.word_len);
        assert_eq!(feedback.len(), self.word_len);

        self.guesses.push(Guess { word, feedback });
    }

    pub fn filter<'a>(&self, words: &'a [String]) -> Vec<&'a String> {
        words
            .iter()
            .filter(|w| w.len() == self.word_len)
            .filter(|w| {
                self.guesses
                    .iter()
                    .all(|g| matches(w, &g.word, &g.feedback))
            })
            .collect()
    }
}

pub fn parse_pattern(pattern: &str) -> Result<Vec<Feedback>> {
    pattern
        .chars()
        .map(|c| {
            Feedback::try_from(c)
                .map_err(|bad| anyhow::anyhow!("invalid pattern character: {}", bad))
        })
        .collect()
}

pub fn matches(word: &str, guess: &str, pattern: &[Feedback]) -> bool {
    let w: Vec<char> = word.chars().collect();
    let g: Vec<char> = guess.chars().collect();

    let mut min_counts: HashMap<char, usize> = HashMap::new();
    let mut max_counts: HashMap<char, usize> = HashMap::new();
    let mut guess_counts: HashMap<char, usize> = HashMap::new();

    // Count letters in the guess
    for &c in &g {
        *guess_counts.entry(c).or_insert(0) += 1;
    }

    // Green + Yellow pass (position + minimum counts)
    for i in 0..w.len() {
        match pattern[i] {
            Feedback::Green => {
                if w[i] != g[i] {
                    return false;
                }
                *min_counts.entry(g[i]).or_insert(0) += 1;
            }
            Feedback::Yellow => {
                if w[i] == g[i] {
                    return false;
                }
                *min_counts.entry(g[i]).or_insert(0) += 1;
            }
            Feedback::Gray => {}
        }
    }

    // Gray pass (establish max counts PER LETTER)
    for (&letter, &guess_count) in &guess_counts {
        let min = *min_counts.get(&letter).unwrap_or(&0);

        // If any instance of this letter is gray, max = min
        let has_gray = g
            .iter()
            .zip(pattern.iter())
            .any(|(&c, &fb)| c == letter && fb == Feedback::Gray);

        if has_gray {
            max_counts.insert(letter, min);
        } else {
            // Otherwise, allow up to guess_count (or unbounded)
            max_counts.insert(letter, usize::MAX);
        }
    }

    // Count actual letters in the candidate word
    let mut actual_counts: HashMap<char, usize> = HashMap::new();
    for &c in &w {
        *actual_counts.entry(c).or_insert(0) += 1;
    }

    // Enforce min constraints
    for (&letter, &min) in &min_counts {
        if actual_counts.get(&letter).unwrap_or(&0) < &min {
            return false;
        }
    }

    // Enforce max constraints
    for (&letter, &max) in &max_counts {
        if actual_counts.get(&letter).unwrap_or(&0) > &max {
            return false;
        }
    }

    true
}

pub fn filter_words<'a>(words: &'a [String], guess: &str, pattern: &[Feedback]) -> Vec<&'a String> {
    words
        .iter()
        .filter(|w| w.len() == guess.len())
        .filter(|w| matches(w, guess, pattern))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_solver_state_multiple_guesses_compound() {
        let words = vec![
            "dusky".to_string(),
            "dusty".to_string(),
            "dumpy".to_string(),
            "daisy".to_string(),
        ];

        let mut state = SolverState::new(5);

        // DAISY → GXXYG
        state.add_guess("daisy".to_string(), feedback_vec(&[2, 0, 0, 1, 2]));

        let remaining = state.filter(&words);
        assert_eq!(remaining.len(), 2);

        // DUSTY → GGXGG
        // S now gray → max S = 0
        state.add_guess("dusty".to_string(), feedback_vec(&[2, 2, 0, 2, 2]));

        let remaining = state.filter(&words);
        assert!(remaining.is_empty());
    }

    #[test]
    fn test_yellow_letter_wrong_position() {
        let words = vec![
            "crate".to_string(),
            "trace".to_string(),
            "react".to_string(),
        ];

        let guess = "crate";
        let pattern = feedback_vec(&[
            0, // C gray
            1, // R yellow
            0, // A gray
            0, // T gray
            0, // E gray
        ]);

        let filtered = filter_words(&words, guess, &pattern);

        // All candidates violate gray constraints
        assert!(filtered.is_empty());
    }

    #[test]
    fn test_green_overrides_gray_elsewhere() {
        let words = vec![
            "sassy".to_string(),
            "gassy".to_string(),
            "class".to_string(),
        ];

        let guess = "sassy";
        let pattern = feedback_vec(&[
            2, // S green
            0, // A gray
            0, // S gray
            0, // S gray
            2, // Y green
        ]);

        let filtered = filter_words(&words, guess, &pattern);

        assert!(filtered.is_empty());
    }

    #[test]
    fn test_repeated_letter_gray_respects_min_count() {
        let words = vec![
            "label".to_string(),
            "cello".to_string(),
            "helot".to_string(),
            "pilot".to_string(),
        ];

        let guess = "allot";
        let pattern = feedback_vec(&[
            0, // A gray
            1, // L yellow
            0, // L gray
            0, // O gray
            0, // T gray
        ]);

        let filtered = filter_words(&words, guess, &pattern);
        assert!(filtered.is_empty());
    }

    fn feedback_vec(pattern: &[u8]) -> Vec<Feedback> {
        pattern
            .iter()
            .map(|&b| match b {
                0 => Feedback::Gray,
                1 => Feedback::Yellow,
                2 => Feedback::Green,
                _ => panic!("Invalid feedback"),
            })
            .collect()
    }

    #[test]
    fn test_filter_words_basic() {
        let words = vec![
            String::from("apple"),
            String::from("apply"),
            String::from("angle"),
            String::from("ample"),
        ];
        let guess = "apple";
        let pattern = feedback_vec(&[2, 2, 2, 2, 2]); // All green
        let filtered = filter_words(&words, guess, &pattern);
        assert_eq!(filtered, vec![&String::from("apple")]);
    }

    #[test]
    fn test_filter_words_length() {
        let words = vec![
            String::from("apple"),
            String::from("apples"),
            String::from("appl"),
        ];
        let guess = "apple";
        let pattern = feedback_vec(&[2, 2, 2, 2, 2]);
        let filtered = filter_words(&words, guess, &pattern);
        assert_eq!(filtered, vec![&String::from("apple")]);
    }
}
