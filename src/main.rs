use anyhow::Result;
use clap::Parser;

use wordle_grep::{
    args::Args,
    scoring::score_and_sort,
    solver::{filter_words, parse_pattern},
    wordlist::load_words,
};

fn main() -> Result<()> {
    let args = Args::parse();

    let guess = args.guess.to_lowercase();
    let pattern = args.pattern.to_uppercase();

    if guess.len() > 5 {
        anyhow::bail!("guess and pattern length must not exceed 5");
    }
    if guess.len() != pattern.len() {
        anyhow::bail!("guess and pattern must be the same length");
    }

    let words = load_words()?;
    let pattern = parse_pattern(&args.pattern)?;
    let matches = filter_words(&words, &guess, &pattern);
    let scored = score_and_sort(&matches);

    for (word, score) in scored {
        println!("{word} ({score})");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::process::Command;

    #[test]
    fn test_guess_and_pattern_same_length() {
        let output = Command::new("cargo")
            .args(["run", "--", "--guess", "apple", "--pattern", "22222"])
            .output()
            .expect("failed to execute process");
        assert!(output.status.success());
    }

    #[test]
    fn test_guess_and_pattern_different_length() {
        let output = Command::new("cargo")
            .args(["run", "--", "--guess", "apple", "--pattern", "2222"])
            .output()
            .expect("failed to execute process");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("guess and pattern must be the same length"));
    }

    #[test]
    fn test_guess_and_pattern_too_long() {
        let output = Command::new("cargo")
            .args(["run", "--", "--guess", "apples", "--pattern", "222222"])
            .output()
            .expect("failed to execute process");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("guess and pattern length must not exceed 5"));
    }
}
