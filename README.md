# wordle-grep 🟩🟨⬛

A fast, no-BS Wordle helper written in Rust.

`wordle-grep` filters and ranks possible Wordle answers based on a guess and a feedback pattern (`G`, `Y`, `X`). It respects **actual Wordle rules** (yes, including repeated letters) and sorts results by letter frequency so you’re not eyeballing a raw list like an animal.

If you’ve ever thought “I could just grep this,” this is that — but done correctly.

## Features

* Proper Wordle constraint handling

  * `G` = correct letter, correct position
  * `Y` = correct letter, wrong position
  * `X` = letter excluded *unless required elsewhere*
* Automatic wordlist download (cached locally)
* Clap-based CLI (because we have standards)
* Frequency-based sorting of candidates
* Fast enough to spam between guesses without thinking

## Installation

Clone the repo and build it like a normal Rust project:

```
git clone https://github.com/yourname/wordle-grep
cd wordle-grep
cargo build --release
```

Or just run it directly:

```
cargo run -- <guess> <pattern>
```

On first run, it will download a standard Wordle wordlist and cache it as `words.txt`.

## Usage

Basic invocation:

```
wordle-grep <guess> <pattern>
```

Example:

```
wordle-grep crane GXXYX
```

Output looks like:

```
irone (312)
satel (298)
crone (291)
```

Higher scores mean the word contains letters that appear more frequently across remaining candidates. Start from the top unless you enjoy wasting guesses.

### Pattern Rules

* `G` — green: letter must match exactly at this position
* `Y` — yellow: letter must exist elsewhere, but **not here**
* `X` — gray: letter must not appear **beyond what’s already required**

This is **not** naive grep logic. Repeated-letter edge cases are handled correctly.

## How Scoring Works

1. Count how often each letter appears across all remaining valid candidates
2. Score each word by summing the frequencies of its **unique letters**
3. Sort descending

Why unique letters? Because double letters don’t magically give you more information, and pretending otherwise is cope.

## Wordlist

The default wordlist is downloaded automatically from a public Wordle list and cached locally. All words are lowercase and filtered by length at runtime.

Future versions will probably let you override this, but for now: it Just Works™.

## Why This Exists

* Because eyeballing lists sucks
* Because most Wordle solvers cheat *way* harder than this
* Because writing a tiny Rust tool is more fun than doomscrolling

## Future Ideas (a.k.a. obvious next steps)

* Multiple guesses in one run
* Different scoring modes (`solve` vs `explore`)
* Colorized output
* Precomputed frequency caching
* Optional hard mode enforcement

## License

Do whatever you want with it. If it helps you win Wordle and feel superior for 30 seconds, mission accomplished.

If you want, next step is adding **multi-guess accumulation** so you don’t re-run the program like a gremlin after every turn. Or we can make it output ANSI-colored words and feel powerful.
