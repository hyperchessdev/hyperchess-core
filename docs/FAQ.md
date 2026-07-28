# Frequently asked questions

## About the game

**Why a 12×12 board?**
An 8×8 board with only new pieces added tends to saturate quickly — strong engines and players
re-derive orthodox-adjacent theory fast. A larger board with two long-range jumping pieces
meaningfully expands the action space and time-to-saturation, which is also what makes it an
interesting benchmark for game AI, not just a new opening book for chess engines. See
[`docs/PROJECT-STORY.md`](PROJECT-STORY.md) for how the board size was actually arrived at (a
physical-board game with a ten-year-old, not a theoretical design exercise).

**What do the Eagle and the Hawk do?**
The Eagle jumps orthogonally up to four squares, ignoring intervening pieces; the Hawk does the
same diagonally. Both can land on an empty square or capture, and because they jump, a piece
can't block their line of attack the way it can block a rook or bishop — see
[`docs/hyperchess-laws.md#3-movement`](hyperchess-laws.md#3-movement).

**Why do pieces keep an "identity" instead of just a type?**
Faithful history, research-quality replay, and unambiguous analysis on a board with more pieces
and more promotion options than orthodox chess. Full rationale in
[Why identity-bearing pieces](IDENTITY-PIECES.md).

## About the engine

**Is this related to the actual Stockfish project or organization?**
No. HyperChess is an independent project that openly re-engineers Stockfish's (and
Fairy-Stockfish's) published techniques and ideas in Rust for a different board and move
space. It is not endorsed by, affiliated with, or a fork of the Stockfish organization. See
[`ACKNOWLEDGMENTS.md`](../ACKNOWLEDGMENTS.md) and
[`docs/ATTRIBUTION-AND-TRUST.md`](ATTRIBUTION-AND-TRUST.md).

**Is it really "zero dependency"?**
The engine core — `hyperchess-eval`, `hyperchess-rules`, `hyperchess-search` — builds with no
external crates by default: `cargo tree -p hyperchess-search -e normal` shows nothing beyond
`search → rules → eval`. The `wasm` feature adds one crate (`js-sys`, for the browser
wall-clock); the driver and binding crates naturally carry their own CLI/server/wasm-bindgen
dependencies, since those aren't the "audit surface" claim. See
[`README.md#zero-dependencies`](../README.md#zero-dependencies).

**Does it need a GPU?**
No. The default build is pure-CPU. CUDA acceleration (`hyperchess-search-cuda`) is optional,
source-only, and never published to crates.io — see
[`docs/ARCHITECTURE.md#build-variants`](ARCHITECTURE.md#build-variants).

**How strong is it?**
There's no established Elo/CCRL-style rating pool for a brand-new variant yet. `perft`
correctness checks and reproducible, fixed-node engine-vs-engine matches (see
[`docs/search-architecture.md#tuning-guide`](search-architecture.md#tuning-guide)) are today's
tools for comparing strength or catching regressions.

**Can I plug in my own bot?**
Yes — anything that speaks the supported UCI subset can play against or alongside the engine.
See [UCI integration](UCI.md).

## About formats and compatibility

**Is HFEN compatible with orthodox FEN?**
No, and it isn't meant to be — same conceptual fields, extended for a 12×12 board and a
different starting position. Don't feed HFEN to an orthodox-FEN parser or vice versa. See
[Formats](FORMATS.md).

**Why is there no `x` for captures in HPGN-I?**
The identity prefix (`M:`, `m:`, etc.) already tells you which piece moved; a redundant capture
marker isn't needed since the rules engine validates destination occupancy. HSAN, the
human-readable notation, does use `x` — see [Formats](FORMATS.md).

## About licensing and provenance

**What license is this under?**
`GPL-3.0-or-later` across the engine and every SDK package — see
[`README.md#license`](../README.md#license) for the recommended out-of-process integration
pattern (Web Worker + `postMessage`, or the REST driver over HTTP) if you're embedding this in
a proprietary application. Dual-licensing is available on request per the README.

**Did AI write this codebase?**
AI tools (Claude, Codex, Gemini) were used, under continuous human direction, as drafting and
implementation aids over roughly three months at an estimated cost of US$5,000. They are not
authors of record and have no authority over licensing, correctness claims, or project
direction — every file is human-reviewed. Full disclosure:
[`docs/ATTRIBUTION-AND-TRUST.md`](ATTRIBUTION-AND-TRUST.md).

**I found a missing or wrong credit — what do I do?**
Please tell us. Correcting attribution is a welcome, first-class contribution — see
[`docs/ATTRIBUTION-AND-TRUST.md`](ATTRIBUTION-AND-TRUST.md) for how to report it, or open a
public issue if the concern isn't sensitive.

## About contributing

**I'm not a Rust programmer — can I still help?**
Yes. Documentation, UI/SDK work, test positions, bot tournaments, and research reproductions
are all real contributions — see
[`CONTRIBUTING.md#ways-to-contribute`](../CONTRIBUTING.md#ways-to-contribute).

**Will you accept an AI-generated pull request?**
Only if there's a named, accountable human behind it who can explain, test, and defend the
change in review — not an autonomous submission. See
[`CONTRIBUTING.md`](../CONTRIBUTING.md) and
[`docs/ATTRIBUTION-AND-TRUST.md`](ATTRIBUTION-AND-TRUST.md).

---

Question not answered here? Open a [Discussion](../../discussions) — a good FAQ addition is a
welcome contribution in its own right.
