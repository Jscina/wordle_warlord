# wordle-grep 🟩🟨⬛

A Wordle solver written in Rust, with an interactive terminal UI.

`wordle-grep` models _actual_ Wordle rules — including repeated letters, per-guess min/max constraints, and multi-guess compounding and surfaces the remaining solution space as it collapses.

This started as “I could just grep this” and turned into “okay, let’s do it properly.”

---

## What This Is (and Isn’t)

**This is:**

- A Wordle solver engine with real semantics
- A stateful, interactive terminal UI
- Deterministic, test-backed logic
- Honest about contradictions (empty results mean _you’re dead_)

If the solver says “no solutions,” that’s not a bug that’s Wordle telling you you fucked up earlier.

---

## Features

- **Correct Wordle constraint handling**
  - Green / Yellow establish **minimum letter counts**
  - Gray establishes **maximum letter counts per guess**
  - Repeated-letter edge cases handled correctly

- **Stateful solving**
  - Constraints accumulate across guesses
  - Later guesses can retroactively invalidate earlier assumptions (as in real Wordle)

- **Interactive terminal UI**
  - Colored tiles (🟩🟨⬛ vibes)
  - Live candidate narrowing
  - Immediate visibility into forced states

- **Frequency-based suggestion ranking**
  - Scores candidates by informational value
  - No double-letter inflation

- **Test-backed solver**
  - Repeated-letter cases
  - Multi-guess compounding
  - Known failure modes covered

---

## Installation

Clone and build like a normal Rust app:

```bash
git clone https://github.com/yourname/wordle-grep
cd wordle-grep
cargo build --release
```

Or just run it:

```bash
cargo run
```

On first launch, it will download and cache a standard Wordle wordlist automatically.

---

## Usage

When you run it:

```bash
cargo run
```

You get an interactive session where you:

1. Enter guesses as:

   ```
   <guess> <pattern>
   ```

   Example:

   ```
   daisy GXXYG
   ```

2. The solver updates immediately:
   - Guesses render as colored tiles
   - Remaining candidates update live
   - Suggestions are ranked by usefulness

3. Repeat until:
   - You win
   - Or the solver correctly tells you there is no valid solution

### Pattern Rules

- `G` — green: correct letter, correct position
- `Y` — yellow: letter exists, wrong position
- `X` — gray: letter does **not** appear beyond what’s already justified in that guess

Lowercase patterns are accepted.

---

## How Suggestions Are Scored

1. Count how often each letter appears across the remaining valid candidates
2. Score each candidate by the **sum of its unique letter frequencies**
3. Sort descending

Why unique letters?
Because repeating letters doesn’t give you new information, and pretending otherwise is cope.

---

## Wordlist

- Downloaded automatically on first run
- Cached locally
- Lowercase
- Filtered by word length at runtime

No configuration needed. It just works.

---

## Why This Exists

- Because eyeballing candidate lists sucks
- Because most Wordle solvers are subtly wrong
- Because repeated-letter logic is a graveyard
- Because building a correct solver is more satisfying than doomscrolling
- Because someone was too confident about a solve in two

---

## Known Behavior (Read This)

- The solver **can return zero candidates**
- This means the constraints are contradictory
- This can happen in real Wordle
- The solver is not broken when this happens

If you don’t like that, you don’t want a correct solver.

---

## License

Do whatever you want with it.

If it helps you win Wordle and feel briefly superior, that’s on you.
If it makes someone else mad, that’s a bonus.
