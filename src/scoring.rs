use std::collections::{HashMap, HashSet};

pub fn score_and_sort(words: &[&String]) -> Vec<(String, usize)> {
    let mut freq: HashMap<char, usize> = HashMap::new();

    for word in words {
        for c in word.chars() {
            *freq.entry(c).or_insert(0) += 1;
        }
    }

    let mut scored: Vec<(String, usize)> = words
        .iter()
        .map(|word| {
            let unique: HashSet<char> = word.chars().collect();
            let score = unique.iter().map(|c| freq[c]).sum();
            ((*word).clone(), score)
        })
        .collect();

    scored.sort_by(|a, b| b.1.cmp(&a.1));
    scored
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_score_and_sort_basic() {
        let words = vec![
            String::from("apple"),
            String::from("grape"),
            String::from("peach"),
            String::from("plumb"),
        ];
        let word_refs: Vec<&String> = words.iter().collect();
        let scored = score_and_sort(&word_refs);

        // All words should be present
        let scored_words: Vec<String> = scored.iter().map(|(w, _)| w.clone()).collect();
        for w in &words {
            assert!(scored_words.contains(w));
        }
        // Sorted by score descending
        for i in 1..scored.len() {
            assert!(scored[i - 1].1 >= scored[i].1);
        }
    }

    #[test]
    fn test_score_and_sort_unique_letters() {
        let words = vec![String::from("abcde"), String::from("aaaaa")];
        let word_refs: Vec<&String> = words.iter().collect();
        let scored = score_and_sort(&word_refs);

        // "abcde" should have a higher score than "aaaaa"
        assert!(scored[0].0 == "abcde");
        assert!(scored[0].1 > scored[1].1);
    }

    #[test]
    fn test_score_and_sort_empty() {
        let words: Vec<String> = vec![];
        let word_refs: Vec<&String> = words.iter().collect();
        let scored = score_and_sort(&word_refs);
        assert!(scored.is_empty());
    }
}
