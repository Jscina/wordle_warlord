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

    // Green + Yellow pass
    for i in 0..w.len() {
        match pattern[i] {
            Feedback::Green => {
                if w[i] != g[i] {
                    return false;
                }
                *min_counts.entry(g[i]).or_insert(0) += 1;
            }
            Feedback::Yellow => {
                if w[i] == g[i] || !w.contains(&g[i]) {
                    return false;
                }
                *min_counts.entry(g[i]).or_insert(0) += 1;
            }
            Feedback::Gray => {}
        }
    }

    // Gray pass
    for i in 0..w.len() {
        if pattern[i] == Feedback::Gray {
            let letter = g[i];
            let required = *min_counts.get(&letter).unwrap_or(&0);
            let actual = w.iter().filter(|&&c| c == letter).count();

            if actual > required {
                return false;
            }
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
