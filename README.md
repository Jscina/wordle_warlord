# wordle-warlord 🟩🟨⬛

A Wordle solver **and** local Wordle game written in Rust, with an interactive terminal UI.

`wordle-warlord` models actual Wordle rules including repeated letters, per-guess min/max constraints, and multi-guess compounding and lets you either:

- solve real Wordle puzzles, or
- play Wordle locally in your terminal.

What started as “I could just grep this” turned into “okay, let’s actually do it right.”

## Solver Mode

![Solver Mode](assets/solver-mode.png)

## Game Mode

![Game Mode](assets/game-mode.png)

---

## What This Is (and Isn’t)

**This is:**

- A correct Wordle solver engine
- A playable local Wordle clone
- An interactive terminal UI
- Deterministic, test-backed solving logic
- Honest about contradictions

If the solver says “no solutions,” that’s not a bug it means earlier constraints contradict reality.

In other words: Wordle says you messed up.

This is not:

- A bot that plays Wordle online for you
- A cheat that instantly reveals the answer
- An automated solver that plays without input
- A mathematically optimal solver minimizing guesses
- Guaranteed to suggest the perfect next move
- A browser plugin or Wordle scraper

It helps you reason about the puzzle it doesn’t play it for you.

## Features

### 🧠 Correct Wordle constraint handling

- Green / Yellow establish **minimum letter counts**
- Gray establishes **maximum counts per guess**
- Repeated-letter edge cases handled correctly
- Constraints compound across guesses like real Wordle

---

### 🎮 Play Wordle locally

- Random solution selection
- Standard 6-guess gameplay
- Colored tile feedback
- Solver suggestions available mid-game if you want help

Play normally, or cheat responsibly.

---

### 🔎 Solver mode

Enter guesses manually and narrow solutions interactively.

- Colored guess history
- Live candidate filtering
- Ranked suggestions
- Undo support
- Constraint visualization

Great for solving real NYT puzzles.

---

### 📊 Live analysis panels

Solver shows:

- Letter frequency breakdown
- Position likelihoods
- Active constraints
- Remaining solution pool stats

So you can actually see the solution space collapse.

---

### 🧪 Test-backed solver

Solver logic includes tests for:

- Repeated-letter behavior
- Multi-guess constraint interaction
- Known Wordle edge cases

If results are empty, the constraints are wrong — not the code.

---

## Manual Installation

Clone and build normally:

```bash
git clone https://github.com/jscina/wordle-warlord.git
cd wordle-warlord
cargo build --release
```

Or just run it:

```bash
cargo run
```

On first run, the app downloads and caches a Wordle wordlist automatically.

---

## Usage

Launch:

```bash
cargo run
```

You start in **solver mode**.

---

### Solver Mode

Enter guesses like:

```
<guess> <pattern>
```

Example:

```
daisy GXXYG
```

Pattern rules:

- `G` = correct letter, correct position
- `Y` = correct letter, wrong position
- `X` = letter not present beyond justified counts

Lowercase also works.

---

### Game Mode

Press:

```
Ctrl+G
```

to start a local Wordle game.

Gameplay:

- Type a guess
- Press Enter
- Receive colored feedback
- Solve within 6 guesses

Press Enter after game over to start a new round.

Return to solver mode with:

```
Ctrl+S
```

---

## Controls

| Key    | Action           |
| ------ | ---------------- |
| Enter  | Submit guess     |
| Ctrl+G | Start game mode  |
| Ctrl+S | Return to solver |
| Ctrl+Z | Undo last guess  |
| Ctrl+Q | Quit             |

---

## Suggestion Ranking

Suggestions are ranked by informational value:

1. Count letter frequency among remaining solutions
2. Score words by sum of **unique letter frequencies**
3. Sort descending

Repeated letters don't give extra information, so they aren't rewarded.

Information beats vibes.

---

## Wordlists

The app downloads and caches a Wordle-compatible wordlist automatically.

Words are:

- Lowercase
- Filtered by length
- Used for both solving and gameplay

No configuration required.

---

## Known Behavior

The solver can legitimately return zero candidates.

This means:

- constraints conflict
- or feedback was entered incorrectly

That can happen in real Wordle.

The solver is not broken when this occurs.

---

## Why This Exists

Because:

- manually tracking constraints sucks
- correct solvers are fun to build
- and someone was way too confident about a solve in two

---

## License

Do whatever you want.

If it helps you win Wordle and feel superior, enjoy it.

If it annoys someone, even better.
