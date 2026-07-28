# Acknowledgments

HyperChess Core exists because generations of engine authors, researchers, and open-source
maintainers shared their work. Thank you.

## Chess engines & the technique canon

- **[Stockfish](https://stockfishchess.org)** — the reference open-source chess engine. The search
  technique family this engine implements — principal variation search, null-move pruning,
  late-move reductions, killer/history/countermove ordering, history gravity, aspiration
  windows, futility pruning — was refined and proven in Stockfish and its community. Our GPLv3
  licensing follows its convention deliberately.
- **[Fairy-Stockfish](https://fairy-stockfish.github.io/)** — proof that variant chess deserves
  first-class engines, and an inspiration for building one for HyperChess.
- **[Chess Programming Wiki](https://www.chessprogramming.org/)** — the indispensable encyclopedia
  of every technique above; our search documentation deliberately uses its vocabulary so readers
  can cross-reference.
- **[Pleco](https://github.com/pleco-rs/Pleco)** and the Rust chess ecosystem — for demonstrating
  idiomatic bitboard engines in Rust.

## Monte Carlo Tree Search

- **Rémi Coulom** — *Efficient Selectivity and Backup Operators in Monte-Carlo Tree Search*
  (2006), the founding MCTS paper.
- **Levente Kocsis & Csaba Szepesvári** — *Bandit based Monte-Carlo Planning* (2006), the UCT
  selection formula our `MctsSearcher` uses.
- **The AlphaZero line** (Silver et al.) — for virtual loss and batched leaf evaluation, which
  shaped our GPU-friendly `mcts_with_eval_bounded` design.

## Algorithms & data structures

- **Zobrist hashing** (Albert Zobrist, 1970) — position keys for the transposition table.
- **MVV-LVA and Static Exchange Evaluation** — classical capture-ordering techniques from the
  computer-chess literature.
- **Negamax with alpha-beta** (Knuth & Moore's formalization) — the backbone of the search.

## Tools & ecosystem

- **Rust** — the language that lets one codebase serve native, server, and browser targets safely.
- **wasm-bindgen / wasm-pack** — the bridge that puts the full engine in a browser.
- **rand**, **js-sys**, and the crates.io ecosystem.
- **TypeScript**, **pnpm**, **turbo**, **vitest**, **esbuild** — the SDK toolchain.
- **Rust CUDA** — GPU experimentation in the optional CUDA crate.

## The variant

- The **FIDE Laws of Chess** — the foundation the HyperChess laws extend.
- Everyone who playtested HyperChess and shaped the Eagle/Hawk rules into their current form.

## AI-assisted development

Claude, Codex, and Gemini were used, under continuous human direction, as drafting,
translation/re-engineering, and implementation aids across roughly three months of development,
at an estimated cost of **US$5,000**. They assisted analysis, code, and documentation; they are
not authors of record and have no authority over licensing, correctness claims, or project
direction — every change is human-reviewed and human-owned. Full disclosure and the standard we
hold AI-assisted contributions to: [`docs/ATTRIBUTION-AND-TRUST.md`](docs/ATTRIBUTION-AND-TRUST.md).

---

*If your work is used here and missing from this list, that's a bug — please open a PR.*
