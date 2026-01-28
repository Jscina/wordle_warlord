use anyhow::Result;
use clap::Parser;
use std::io::{self, Write};

use wordle_grep::{
    args::Args,
    scoring::score_and_sort,
    solver::{SolverState, parse_pattern},
    wordlist::load_words,
};

fn main() -> Result<()> {
    let args = Args::parse();
    let words = load_words()?;

    let mut state = SolverState::new(5);

    if args.interactive {
        return interactive_loop(&mut state, &words);
    }

    for pair in args.guess.chunks_exact(2) {
        let guess = pair[0].to_lowercase();
        let feedback = parse_pattern(&pair[1])?;
        state.add_guess(guess, feedback);
    }

    let remaining = state.filter(&words);
    let scored = score_and_sort(&remaining);

    for (word, score) in scored {
        println!("{word} ({score})");
    }

    Ok(())
}

fn interactive_loop(state: &mut SolverState, words: &[String]) -> anyhow::Result<()> {
    let stdin = io::stdin();

    loop {
        print!("guess> ");
        io::stdout().flush()?;

        let mut line = String::new();
        if stdin.read_line(&mut line)? == 0 {
            break; // EOF
        }

        let parts: Vec<_> = line.split_whitespace().collect();
        if parts.len() != 2 {
            eprintln!("expected: WORD PATTERN (or Ctrl+D to exit)");
            continue;
        }

        let guess = parts[0].to_lowercase();
        let feedback = parse_pattern(parts[1])?;

        state.add_guess(guess, feedback);

        let remaining = state.filter(words);
        let scored = score_and_sort(&remaining);

        println!("remaining: {}", remaining.len());
        for (word, score) in scored.iter().take(10) {
            println!("{word} ({score})");
        }

        if remaining.len() <= 2 {
            println!("⚠ forced state detected");
        }
    }

    Ok(())
}
