use anyhow::{Context, Result};
use rand::seq::SliceRandom;
use reqwest::blocking::get;
use std::fs;
use std::path::Path;

const WORDLIST_URL: &str = "https://raw.githubusercontent.com/tabatkins/wordle-list/main/words";
const WORDLIST_PATH: &str = "words.txt";

pub fn load_words() -> Result<Vec<String>> {
    ensure_wordlist()?;

    let text = fs::read_to_string(WORDLIST_PATH).context("failed to read wordlist")?;

    Ok(text.lines().map(|s| s.to_string()).collect())
}

pub fn select_random_word(words: &[String], word_len: usize) -> Result<String> {
    let filtered: Vec<&String> = words.iter().filter(|w| w.len() == word_len).collect();

    if filtered.is_empty() {
        return Err(anyhow::anyhow!("no {}-letter words available", word_len));
    }

    let selected = filtered
        .choose(&mut rand::thread_rng())
        .ok_or_else(|| anyhow::anyhow!("failed to select random word"))?;

    Ok(selected.to_string())
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
