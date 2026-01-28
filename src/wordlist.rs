use anyhow::{Context, Result};
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

fn ensure_wordlist() -> Result<()> {
    if Path::new(WORDLIST_PATH).exists() {
        return Ok(());
    }

    eprintln!("downloading wordlist...");
    let text = get(WORDLIST_URL)?.error_for_status()?.text()?;

    fs::write(WORDLIST_PATH, text)?;
    Ok(())
}
