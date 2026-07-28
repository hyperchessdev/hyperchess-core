# The story and vision

HyperChess Core did not start as a benchmark or a product pitch.

In 2025, its creator and his ten-year-old son, **Arda Kuzey**, designed a new chess variation
while on vacation, and started playing it on a physical board. It didn't take long to notice
that classical chess strategy broke down almost immediately: the new rules — a larger board,
two new long-range jumping pieces — created a position space that ordinary intuition and
memorized theory didn't map onto cleanly. The game felt different enough, and complex enough,
that building a custom engine became the only honest way to actually understand it.

## From a physical board to an open laboratory

As the project developed, the idea expanded past "a new variant with a bot to play against."
The underlying observation was broader: in nature, every living creature learns to navigate its
environment through play. That raised a real question — could play also be a serious
foundation for human–AI collaboration, not just a source of test cases for reinforcement
learning?

HyperChess is an attempt to make that question concrete: an open, deterministic,
high-complexity game — large and novel enough to resist quick saturation — built and
maintained fully in the open, so that players, engine authors, and researchers can use it as a
shared laboratory for studying how humans and machines learn, decide, and get better together.

## Why open, and why in public

- **Reproducibility over secrecy.** A rules engine, a search stack, and eventually a dataset
  that anyone can inspect, rebuild, and challenge is worth more to this question than a closed
  demo.
- **Attribution as a first principle.** HyperChess re-engineers decades of open computer-chess
  work rather than claiming to invent it — see [`ACKNOWLEDGMENTS.md`](../ACKNOWLEDGMENTS.md) and
  [`docs/ATTRIBUTION-AND-TRUST.md`](ATTRIBUTION-AND-TRUST.md). The project's own legitimacy
  depends on that distinction staying honest.
- **A commons, not a product.** The ambition is not to declare a finished theory of a new game.
  It is to build the rules, the engine, the datasets, the tools, and the culture in public;
  invite critique early and often; and leave a durable commons for the next generation of
  players and builders — the way Stockfish and Fairy-Stockfish left one for computer chess.

## Where the pieces come from

The Eagle and the Hawk are not abstractions chosen for balance-sheet symmetry — they're the
pieces that came out of that first physical-board game, kept in the engine exactly as
originally played. Their jumping, occupancy-independent movement is what makes HyperChess
tactically distinct from an 8×8 game on a bigger board; see
[`docs/hyperchess-laws.md`](hyperchess-laws.md) for the precise rules and
[`docs/search-architecture.md`](search-architecture.md#move-ordering-srcsearchorderingrs) for
how the search itself accounts for the fact that their checks can't be blocked. See
[Why pieces carry identity](IDENTITY-PIECES.md) for how the project chose to keep faith with
that original game's history even in engine-generated notation.

## What "success" looks like here

Not a fixed roadmap milestone, but a durable pattern: a stranger can play HyperChess in under a
minute, understand why it's different within a game or two, and — if they want to — go on to
read the code, fix a bug, write a bot, or contribute to the first public body of HyperChess
theory. See [`docs/ROADMAP.md`](ROADMAP.md) for the concrete near-term steps toward that, and
[`CONTRIBUTING.md`](../CONTRIBUTING.md) if that stranger is you.

---

*Inspired by Arda Kuzey, and built with deep gratitude for everyone who has contributed to the
rich history of chess through the ages.*
