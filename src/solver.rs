use anyhow::{Result, bail};
use std::collections::HashMap;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Feedback {
    Green,
    Yellow,
    Gray,
}

pub fn parse_pattern(pattern: &str) -> Result<Vec<Feedback>> {
    pattern
        .chars()
        .map(|c| match c {
            'G' => Ok(Feedback::Green),
            'Y' => Ok(Feedback::Yellow),
            'X' => Ok(Feedback::Gray),
            _ => bail!("invalid pattern character: {}", c),
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
