use anyhow::{Context, Result};
use rand::seq::IndexedRandom;
use reqwest::blocking::get;
use std::fs;
use std::path::Path;

const WORDLIST_URL: &str = "https://raw.githubusercontent.com/tabatkins/wordle-list/main/words";
const SOLUTIONS_URL: &str = "https://gist.githubusercontent.com/cfreshman/a03ef2cba789d8cf00c08f767e0fad7b/raw/wordle-answers-alphabetical.txt";

const WORDLIST_PATH: &str = "words.txt";
const SOLUTIONS_PATH: &str = "solutions.txt";

pub fn load_words() -> Result<Vec<String>> {
    ensure_file(WORDLIST_PATH, WORDLIST_URL)?;

    let text = fs::read_to_string(WORDLIST_PATH).context("failed to read wordlist")?;

    Ok(text.lines().map(|s| s.to_string()).collect())
}

pub fn load_solutions() -> Result<Vec<String>> {
    ensure_file(SOLUTIONS_PATH, SOLUTIONS_URL)?;

    let text = fs::read_to_string(SOLUTIONS_PATH).context("failed to read solutions")?;

    Ok(text.lines().map(|s| s.to_string()).collect())
}

pub fn select_random_word(words: &[String], word_len: usize) -> Result<String> {
    let filtered: Vec<&String> = words.iter().filter(|w| w.len() == word_len).collect();

    if filtered.is_empty() {
        return Err(anyhow::anyhow!("no {}-letter words available", word_len));
    }

    let selected = filtered
        .choose(&mut rand::rng())
        .ok_or_else(|| anyhow::anyhow!("failed to select random word"))?;

    Ok(selected.to_string())
}

fn ensure_file(path: &str, url: &str) -> Result<()> {
    if Path::new(path).exists() {
        return Ok(());
    }

    eprintln!("downloading {}...", path);

    let text = get(url)?.error_for_status()?.text()?;
    fs::write(path, text)?;

    Ok(())
}
