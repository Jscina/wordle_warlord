use anyhow::Result;
use std::collections::HashMap;
use std::convert::TryFrom;

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
            'G' => Ok(Self::Green),
            'Y' => Ok(Self::Yellow),
            'X' => Ok(Self::Gray),
            _ => Err(value),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Guess {
    pub word: String,
    pub feedback: Vec<Feedback>,
}

impl Guess {
    pub fn new(word: String, feedback: Vec<Feedback>) -> Self {
        Self { word, feedback }
    }
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

    pub fn word_len(&self) -> usize {
        self.word_len
    }

    pub fn guesses(&self) -> &[Guess] {
        &self.guesses
    }

    pub fn pop_guess(&mut self) {
        self.guesses.pop();
    }

    pub fn add_guess(&mut self, guess: Guess) {
        assert_eq!(guess.word.len(), self.word_len);
        assert_eq!(guess.feedback.len(), self.word_len);

        self.guesses.push(guess);
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

    if w.len() != g.len() || g.len() != pattern.len() {
        return false;
    }

    // Count letters in candidate
    let mut counts = HashMap::new();
    for &c in &w {
        *counts.entry(c).or_insert(0) += 1;
    }

    // First pass: enforce greens and reduce counts
    for i in 0..w.len() {
        if pattern[i] == Feedback::Green {
            if w[i] != g[i] {
                return false;
            }
            *counts.get_mut(&g[i]).unwrap() -= 1;
        }
    }

    // Second pass: yellows
    for i in 0..w.len() {
        if pattern[i] == Feedback::Yellow {
            if w[i] == g[i] {
                return false;
            }

            match counts.get_mut(&g[i]) {
                Some(c) if *c > 0 => *c -= 1,
                _ => return false,
            }
        }
    }

    // Third pass: grays must have no remaining matches
    for i in 0..w.len() {
        if pattern[i] == Feedback::Gray && matches!(counts.get(&g[i]), Some(c) if *c > 0) {
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

pub fn generate_feedback(target: &str, guess: &str) -> Vec<Feedback> {
    let t: Vec<char> = target.chars().collect();
    let g: Vec<char> = guess.chars().collect();

    let mut result = vec![Feedback::Gray; g.len()];
    let mut counts = HashMap::new();

    for &c in &t {
        *counts.entry(c).or_insert(0) += 1;
    }

    // greens
    for i in 0..g.len() {
        if g[i] == t[i] {
            result[i] = Feedback::Green;
            *counts.get_mut(&g[i]).unwrap() -= 1;
        }
    }

    // yellows
    for i in 0..g.len() {
        if result[i] == Feedback::Green {
            continue;
        }

        if let Some(c) = counts.get_mut(&g[i])
            && *c > 0
        {
            result[i] = Feedback::Yellow;
            *c -= 1;
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn test_solver_state_multiple_guesses_compound() {
        let words = vec![
            "dusky".to_string(),
            "dusty".to_string(),
            "dumpy".to_string(),
            "daisy".to_string(),
        ];

        let mut state = SolverState::new(5);

        // DAISY → GXXYG
        state.add_guess(Guess::new(
            "daisy".to_string(),
            feedback_vec(&[2, 0, 0, 1, 2]),
        ));

        let remaining = state.filter(&words);
        assert_eq!(remaining.len(), 2);

        // DUSTY → GGXGG
        state.add_guess(Guess::new(
            "dusty".to_string(),
            feedback_vec(&[2, 2, 0, 2, 2]),
        ));

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
        let pattern = feedback_vec(&[0, 1, 0, 0, 0]);

        let filtered = filter_words(&words, guess, &pattern);

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
        let pattern = feedback_vec(&[2, 0, 0, 0, 2]);

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
        let pattern = feedback_vec(&[0, 1, 0, 0, 0]);

        let filtered = filter_words(&words, guess, &pattern);

        assert!(filtered.is_empty());
    }

    #[test]
    fn test_filter_words_basic() {
        let words = vec![
            "apple".to_string(),
            "apply".to_string(),
            "angle".to_string(),
            "ample".to_string(),
        ];

        let guess = "apple";
        let pattern = feedback_vec(&[2, 2, 2, 2, 2]);

        let filtered = filter_words(&words, guess, &pattern);

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].as_str(), "apple");
    }

    #[test]
    fn test_filter_words_length() {
        let words = vec![
            "apple".to_string(),
            "apples".to_string(),
            "appl".to_string(),
        ];

        let guess = "apple";
        let pattern = feedback_vec(&[2, 2, 2, 2, 2]);

        let filtered = filter_words(&words, guess, &pattern);

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].as_str(), "apple");
    }

    #[test]
    fn test_generate_feedback_duplicate_letters() {
        let fb = generate_feedback("apple", "allay");

        assert_eq!(
            fb,
            vec![
                Feedback::Green,
                Feedback::Yellow,
                Feedback::Gray,
                Feedback::Gray,
                Feedback::Gray,
            ]
        );
    }
}
