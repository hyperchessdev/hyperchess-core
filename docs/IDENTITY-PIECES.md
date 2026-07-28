# Why HyperChess uses identity-bearing pieces

Classical chess notation usually identifies a piece by type and square. That's enough when a
pawn promotes into an interchangeable queen and analysis only needs the current position.
HyperChess deliberately keeps the **identity of each individual piece** through movement,
capture history, and promotion — every piece present at the start of the game has a stable
letter assigned to it for its entire life, whatever it becomes.

This choice serves three purposes:

1. **Faithful game history.** A record can say precisely which *original* pawn became an Eagle
   or a Hawk, rather than only that "a pawn promoted here."
2. **Research-quality replay.** Datasets can trace an individual piece across an entire game
   instead of inferring identity from board snapshots.
3. **Human-readable analysis.** In a 12×12 game with 20 pieces per side and two new promotion
   options (see [`docs/hyperchess-laws.md`](hyperchess-laws.md)), identity removes ambiguity
   without changing legality at all.

## Two parseable position styles, one engine

`hyperchess-rules` accepts **both** notation styles for a position, and picks between them
automatically — see [`docs/FORMATS.md`](FORMATS.md#hfen-and-hfen-i):

- **HFEN** — type-only piece letters, no identity tracked. What most ad-hoc test positions and
  quick tactical puzzles use.
- **HFEN-I** — every starting piece gets its own letter, detected automatically the moment a
  position string contains an identity-only character.

Identity is metadata carried alongside the board, never an input to move generation. A tool
that doesn't care about identity can always use plain HFEN and plain UCI moves and ignore the
feature entirely — nothing about legality changes either way.

## Where identity shows up in notation

- **HFEN-I** — the position-level identity map; see [`docs/FORMATS.md`](FORMATS.md#hfen-and-hfen-i).
- **HPGN-I** — an optional one-character identity prefix on each move (`M:a3a4` vs. plain
  `a3a4`); see [`docs/FORMATS.md`](FORMATS.md#hpgn-i-identity-aware-game-record).
- **HSAN** — deliberately carries **no** identity; reach for it when readability matters more
  than provenance. See [`docs/hyperchess-laws.md#6-notation`](hyperchess-laws.md#6-notation).

## An intentional trade-off

Identity makes notation richer and variant integrations slightly more demanding to write
correctly — a tool has to actively decide to ignore it, rather than there being nothing to
ignore. In exchange, it gives HyperChess a reliable bridge between play, datasets, and the kind
of longitudinal, per-piece analysis this project cares about — see
[`docs/PROJECT-STORY.md`](PROJECT-STORY.md) for why that mattered from the start, not just to
notation design.
