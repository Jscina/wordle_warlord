use anyhow::{Context, Result};
use clap::Parser;
use reqwest::blocking::get;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

const WORDLIST_URL: &str = "https://raw.githubusercontent.com/tabatkins/wordle-list/main/words";
const WORDLIST_PATH: &str = "words.txt";

#[derive(Parser)]
#[command(author, version, about)]
struct Args {
    /// Your guessed word
    guess: String,

    /// Pattern like GXXYX
    pattern: String,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let guess = args.guess.to_lowercase();
    let pattern = args.pattern.to_uppercase();

    if guess.len() != pattern.len() {
        anyhow::bail!("guess and pattern must be the same length");
    }

    ensure_wordlist()?;
    let words = fs::read_to_string(WORDLIST_PATH).context("failed to read wordlist")?;

    let mut matches_vec = Vec::new();

    for word in words.lines() {
        if word.len() != guess.len() {
            continue;
        }

        if matches(word, &guess, &pattern) {
            matches_vec.push(word.to_string());
        }
    }

    let scored = score_and_sort(&matches_vec);

    for (word, score) in scored {
        println!("{word} ({score})");
    }

    Ok(())
}

fn ensure_wordlist() -> Result<()> {
    if Path::new(WORDLIST_PATH).exists() {
        return Ok(());
    }

    eprintln!("downloading wordlist...");
    let text = get(WORDLIST_URL)?.error_for_status()?.text()?;

    fs::write(WORDLIST_PATH, text)?;
    Ok(())
}

fn score_and_sort(words: &[String]) -> Vec<(String, usize)> {
    let mut freq: HashMap<char, usize> = HashMap::new();

    // Count letter frequency across all candidates
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
            (word.clone(), score)
        })
        .collect();

    // Highest score first
    scored.sort_by(|a, b| b.1.cmp(&a.1));

    scored
}

fn matches(word: &str, guess: &str, pattern: &str) -> bool {
    let w: Vec<char> = word.chars().collect();
    let g: Vec<char> = guess.chars().collect();
    let p: Vec<char> = pattern.chars().collect();

    let mut min_counts: HashMap<char, usize> = HashMap::new();

    // G + Y pass
    for i in 0..w.len() {
        match p[i] {
            'G' => {
                if w[i] != g[i] {
                    return false;
                }
                *min_counts.entry(g[i]).or_insert(0) += 1;
            }
            'Y' => {
                if w[i] == g[i] || !w.contains(&g[i]) {
                    return false;
                }
                *min_counts.entry(g[i]).or_insert(0) += 1;
            }
            'X' => {}
            _ => return false,
        }
    }

    // X pass (proper Wordle logic)
    for i in 0..w.len() {
        if p[i] == 'X' {
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
