# HFEN, HFEN-I, HSAN, and HPGN-I formats

HyperChess uses several complementary text formats, all designed for a 12×12 board,
reproducible replay, and — for the `-I` variants — pieces whose identity persists through
promotion. `hyperchess-rules::notation` and `hyperchess-rules::board::hfen` are the canonical
implementation of all of them — do not hand-write a parser or exporter elsewhere. This document
is the reference `crates/hyperchess-rules/src/notation.rs` itself points to (its own module doc
names `docs/HFEN-I-FORMAT.md` and `docs/HPGN-I-CANONICAL-SPEC.md`, which never existed in this
repository — this file is that reference, consolidated).

| Format | Purpose | Identity-aware? |
| --- | --- | --- |
| **HFEN** | One position, complete state | No |
| **HFEN-I** | One position, complete state, with piece identity | Yes |
| **HSAN** | One move, familiar algebraic style — see [`docs/hyperchess-laws.md#6-notation`](hyperchess-laws.md#6-notation) | No |
| **HPGN-I** | A full game record | Yes |

Standard chess FEN/PGN compatibility is intentionally **not** provided — HyperChess's 12×12
board, its two extra piece types, and identity-tracked promotions have no faithful encoding in
the classic formats. Don't feed HFEN to an orthodox-FEN parser, or vice versa.

## HFEN and HFEN-I

HFEN is the canonical single-position encoding: like orthodox FEN, slash-separated board ranks
followed by side to move, castling rights, en-passant information, the half-move clock, and the
full-move number, extended to a 12×12 board (see
[`docs/hyperchess-laws.md#1-board`](hyperchess-laws.md#1-board)).

**HFEN-I** is the identity-aware variant: every one of the 24 starting pieces per side gets its
own stable letter, which follows that piece through movement, capture history, and promotion —
rather than a type-only letter. The parser (`hyperchess-rules::core::piece_identity`) detects
HFEN-I automatically: a position is treated as identity-aware the moment it contains any
identity-only character (one that isn't also a valid legacy piece letter, per
`is_identity_char`/`is_legacy_piece_char`). Both styles parse into the same `Board` type and are
subject to exactly the same legality rules — identity is metadata carried alongside the board,
never an input to move generation.

Why keep identity at all, when a type-only HFEN is sufficient for legality? See
[Why HyperChess uses identity-bearing pieces](IDENTITY-PIECES.md) — in short: faithful game
history (which original pawn became an Eagle?), research-quality replay (tracing one piece
across a whole game), and unambiguous analysis on a board with more pieces and more promotion
options than orthodox chess.

## HSAN (HyperChess Standard Algebraic Notation)

The familiar, human-readable per-move notation — HyperChess's analogue of orthodox SAN — for
contexts where readability matters more than machine round-tripping. Engine I/O (UCI, the REST
API, CLI exports) generally uses coordinate notation instead (`g3g5`, `c11b11`); HSAN is for
game transcripts a person reads. See
[`docs/hyperchess-laws.md#6-notation`](hyperchess-laws.md#6-notation) for the exact grammar
(piece letters, `x` for captures, `=` for promotion, `+`/`#` for check/mate, `O-O`/`O-O-O` for
castling). HSAN carries **no piece identity** — for a durable, identity-preserving record, use
HPGN-I.

## HPGN-I (identity-aware game record)

`GameRecord` (`hyperchess-rules::notation`) is the canonical entry point for reading and
writing a full HyperChess game. A move is ordinary coordinate UCI, optionally prefixed by the
one-character identity held by the source piece:

```text
1. M:a3a4 m:a10a9 2. N:b3b4 n:b10b9 *
```

`M:a3a4` and plain `a3a4` execute identically — the prefix preserves *which individual piece*
moved, not whether the move was a capture. This makes it possible to follow a single pawn
across the whole game, including through promotion, or disambiguate two otherwise-identical
pieces during replay or analysis. Promotion appends the target piece letter to the coordinate
move (`a11a12q`, `a11a12e` for Eagle, `a11a12h` for Hawk). Standard result markers terminate the
game: `1-0`, `0-1`, `1/2-1/2`, `*`.

Both HPGN-I movetext (`M:a3a4`) and plain UCI (`a3a4`) round-trip through `GameRecord::from_hpgni`
— a parser or downstream tool that doesn't care about identity can simply ignore the prefix.

### File extensions

- **`.hpgni`** — tag pairs plus numbered HPGN-I movetext (`GameRecord::to_hpgni`/`from_hpgni`).
- **`.hfeni`** — every HFEN-I position of the game, one per line, produced by replaying the
  start position through the move list (`GameRecord::to_hfeni`/`positions`) — ideal for training
  data and deterministic replay checks.

`hyperchess play` writes both for every completed game; `--format nnue-plain` additionally
appends a line-oriented `fen`/`move`/`score`/`ply`/`result` record per half-move to a shared
`training.plain` file for neural-network training pipelines.

## Compatibility promise

Formats are versioned by documented behavior, not by wishful compatibility with orthodox chess
notations. Consumers should retain original text where possible, validate everything through
`hyperchess-rules` rather than a hand-rolled parser, and pin an exact engine commit SHA for
research datasets. Propose a format extension publicly (see
[`GOVERNANCE.md`](../GOVERNANCE.md#rfc--design-note-process)) before depending on it downstream.
