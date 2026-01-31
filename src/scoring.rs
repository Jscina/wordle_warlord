use std::collections::{HashMap, HashSet};

pub fn score_and_sort(words: &[&String], solutions: &HashSet<String>) -> Vec<(String, usize)> {
    let mut freq: HashMap<char, usize> = HashMap::new();

    for word in words {
        for c in word.chars() {
            *freq.entry(c).or_insert(0) += 1;
        }
    }

    const SOLUTION_BONUS: usize = 10;

    let mut scored: Vec<(String, usize)> = words
        .iter()
        .map(|word| {
            let unique: HashSet<char> = word.chars().collect();

            let mut score: usize = unique.iter().map(|c| freq[c]).sum();

            if solutions.contains(*word) {
                score += SOLUTION_BONUS;
            }

            ((*word).clone(), score)
        })
        .collect();

    scored.sort_by(|a, b| b.1.cmp(&a.1));
    scored
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn test_score_and_sort_basic() {
        let words = vec![
            String::from("apple"),
            String::from("grape"),
            String::from("peach"),
            String::from("plumb"),
        ];

        let word_refs: Vec<&String> = words.iter().collect();

        // Treat all words as valid solutions for neutrality
        let solutions: HashSet<String> = words.iter().cloned().collect();

        let scored = score_and_sort(&word_refs, &solutions);

        // All words should be present
        let scored_words: Vec<String> = scored.iter().map(|(w, _)| w.clone()).collect();

        for w in &words {
            assert!(scored_words.contains(w));
        }

        // Sorted descending
        for i in 1..scored.len() {
            assert!(scored[i - 1].1 >= scored[i].1);
        }
    }

    #[test]
    fn test_score_and_sort_unique_letters() {
        let words = [String::from("abcde"), String::from("aaaaa")];

        let word_refs: Vec<&String> = words.iter().collect();

        // Only "abcde" is a solution, reinforcing ordering
        let mut solutions = HashSet::new();
        solutions.insert("abcde".to_string());

        let scored = score_and_sort(&word_refs, &solutions);

        assert_eq!(scored[0].0, "abcde");
        assert!(scored[0].1 > scored[1].1);
    }

    #[test]
    fn test_score_and_sort_empty() {
        let words: Vec<String> = vec![];
        let word_refs: Vec<&String> = words.iter().collect();
        let solutions: HashSet<String> = HashSet::new();

        let scored = score_and_sort(&word_refs, &solutions);

        assert!(scored.is_empty());
    }

    #[test]
    fn test_solution_bonus_applied() {
        // Same letter distribution, solution should win
        let words = [String::from("crate"), String::from("trace")];

        let word_refs: Vec<&String> = words.iter().collect();

        let mut solutions = HashSet::new();
        solutions.insert("crate".to_string());

        let scored = score_and_sort(&word_refs, &solutions);

        assert_eq!(scored[0].0, "crate");
    }

    #[test]
    fn solution_words_get_bonus() {
        let words = [String::from("crate"), String::from("probe")];
        let word_refs: Vec<&String> = words.iter().collect();

        let mut solutions = HashSet::new();
        solutions.insert("probe".to_string());

        let scored = score_and_sort(&word_refs, &solutions);

        assert_eq!(scored[0].0, "probe");
    }
}
