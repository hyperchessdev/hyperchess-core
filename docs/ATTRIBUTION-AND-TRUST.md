# Attribution and trust

HyperChess Core is indebted to the people and projects that made modern open computer chess
possible. This document is the project's honesty policy: what is derived versus original, how
AI tools were used in building it, and — most importantly — how to correct us when we get any
of that wrong. In this community attribution is not a footer; it is a condition of trust, and
trust is the actual scarce resource for a small open-source project.

## What is derived, and what is original

We acknowledge Stockfish, Fairy-Stockfish, the Chess Programming Wiki, the MCTS/UCT research
line, and the Rust/WebAssembly ecosystems as major sources of technique, convention, and
comparative reference — the full, specific credit list is in
[`ACKNOWLEDGMENTS.md`](../ACKNOWLEDGMENTS.md). We will not blur the line between "re-engineered
from published technique" and "original to this project":

| Component | Provenance |
| --- | --- |
| Search technique family (PVS, null-move, LMR, killer/history/countermove ordering, aspiration windows, futility pruning, SEE-pruned quiescence) | Re-engineered from well-published computer-chess technique — see [`docs/search-architecture.md`](search-architecture.md) and [`ACKNOWLEDGMENTS.md`](../ACKNOWLEDGMENTS.md) for exact lineage |
| MCTS/UCT searcher | Standard published algorithm (Coulom; Kocsis & Szepesvári), with virtual-loss/batched evaluation informed by the AlphaZero line |
| HFEN, HSAN | Direct structural derivatives of orthodox FEN and SAN, extended for a 12×12 board and the Eagle/Hawk piece set |
| The Eagle, the Hawk, the 12×12 board and its laws, the countermove/raptor-bonus search ordering | Original to this project — see [`docs/hyperchess-laws.md`](hyperchess-laws.md) |

A contributor proposing code, an algorithm, or a data structure derived from another project
must identify the source, its license, and the nature of the adaptation before review — see
[`CONTRIBUTING.md`](../CONTRIBUTING.md).

## AI-assisted development disclosure

**Claude, Codex, and Gemini** were used in a human-directed orchestration workflow across
roughly three months of development, at an estimated cost of **US$5,000**. They assisted
analysis, drafting, translation/re-engineering of published algorithms into this codebase, and
implementation, under continuous human review at every step.

What that means concretely:

- **They are not authors of record.** No authority to waive a license, verify a factual or
  legal claim, accept a pull request, or speak for the project.
- **Nothing merges without human review.** Every file — however first drafted — is read,
  checked, and owned by an accountable human. Generated text or code is never self-verifying.
- **The cost figure is a real, stated estimate**, offered so anyone evaluating this project's
  provenance has the actual number rather than having to guess.
- **This applies to documentation, too** — this document and its neighbors were drafted with
  AI assistance and are disclosed the same way engine code is.

This standard extends to any contribution, not just the founding work: see
[`CONTRIBUTING.md#ai-assisted-contributions`](../CONTRIBUTING.md) for what we expect (and
decline) from AI-assisted pull requests — the same reasoning QEMU and LLVM have applied to
AI-derived contributions, and the reasoning behind curl's 2026 bug-bounty pause over a flood of
low-quality AI-generated reports.

## Why this standard

A small open-source project's real asset is not its code — it's whether people believe what it
tells them. The computer-chess community enforces this unusually strictly: the ICGA banned
Vasik Rajlich and Rybka for life in 2011 over plagiarized code, and Stockfish's own maintainers
have litigated GPL compliance in court. We would rather over-disclose than have someone else
discover an unattributed algorithm or an unreviewed AI-generated claim and have to correct the
record for us.

## Every merged change withstands ordinary engineering review

Provenance, licensing, correctness, tests, maintainability, clear human ownership — every time,
regardless of how a change was drafted. We welcome AI-assisted contributions only when the
submitter can explain, validate, and take responsibility for them in their own words.

## How to report a problem

- **Sensitive concern** (e.g. a licensing dispute, a legal question) — report privately via
  [`SECURITY.md`](../SECURITY.md).
- **Otherwise** — open a public issue or discussion. Correcting the record is a welcome,
  first-class contribution, not an inconvenience.

We will correct any attribution, licensing, or historical claim promptly once verified, and
credit whoever raised it.
