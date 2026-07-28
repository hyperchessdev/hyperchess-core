# Search Architecture

This document explains every search technique in `crates/hyperchess-search`, why it exists, and
the safety invariants that make the engine dependable as a backbone for applications. It is
written to be read top-to-bottom by someone who knows chess programming basics; each section
names the source file that implements it.

## Design principles

1. **One canonical search.** Every entry point — CLI, UCI, REST, WASM, and all the historical
   searcher names (`AlphaBetaSearcher`, `IterativeSearcher`, `GuidedAlphaBeta`, …) — delegates to
   the same `TimedSearcher` (`src/timed.rs`). There is exactly one implementation to test, tune,
   and trust; the engine plays identically everywhere.
2. **Anytime by construction.** The search can be stopped at any moment (wall clock, node cap, or
   an atomic flag) and still returns the best move from the deepest *completed* iteration. This is
   what makes it usable in browsers, where JavaScript cannot interrupt a synchronous WASM call —
   the search watches its own clock.
3. **Never trust a shortcut with the played move.** Speed tricks (TT cutoffs, null-move, LMR,
   futility) may only affect *interior* scores, never inject an unvalidated move at the root.
4. **HyperChess is not 8×8 chess scaled up.** The 12×12 board and the jumping raptors change the
   tactics; the search carries variant-specific knowledge (see the raptor bonus and unblockable
   check handling below).

## The anytime driver (`src/timed.rs`)

`TimedSearcher::search_with_stats` runs **iterative deepening** from depth 1 up to a ceiling
(hard max 64), returning `SearchStats { best_move, completed_depth, nodes, aborted }`.

- **Stop conditions** (`SearchLimits`): any combination of `movetime_ms` (cross-platform
  millisecond clock — `std::time` natively, `js_sys::Date::now()` on wasm32), `node_limit`
  (deterministic, the basis of reproducible tests), and a caller-owned `AtomicBool` (the server
  flips it from a watchdog thread). Checks are amortised: the clock/atomic is consulted every
  2048 nodes; the node cap every node.
- **Aspiration windows** (from depth 4): search a ±50 cp window around the previous iteration's
  score, widening exponentially on a fail. Most iterations land inside the window, raising cutoff
  rates tree-wide; a miss only costs a re-search, never accuracy.
- **Early mate stop:** once a proven mate score appears, deeper iterations cannot improve it, so
  the loop ends.

### The negamax core

Per node, in order:

1. **Abort check / node tick.**
2. **Repetition & rule draws.** Threefold repetition, the scaled 112-move rule (224 half-moves),
   and insufficient material return draw scores — but **never at the root**, which must always
   produce a move. Checkmate takes precedence over the move rule (in check the draw only counts
   if an evasion exists).
3. **Check extension.** In-check nodes search one ply deeper, so forcing sequences resolve
   instead of being cut off at the horizon. This is where the raptors matter: an Eagle/Hawk check
   cannot be answered by interposition, so evasions are few and the extension is cheap yet
   decisive.
4. **Transposition table probe.** Entries store `(key, best_move, score, depth, flag)`. Two
   safety rules: **no TT cutoff at the root** (a key collision must never hand an illegal move to
   the game), and mate scores are **re-based to root distance** on store/probe
   (`value_to_tt`/`value_from_tt` in `src/search/terminal.rs`) so "mate in N" survives
   transpositions at different plies.
5. **Reverse futility** *(Aggressive profile)*: at depth ≤ 3, if the static eval beats beta by a
   120·depth margin, fail high without searching. Guarded: never in check, at the root, or near
   mate scores.
6. **Null-move pruning**: give the opponent a free move at reduced depth (R = 2–3); if we still
   beat beta, the real best move surely would. Guarded against zugzwang (side to move must have a
   non-pawn piece), check, the root, and mate bounds — and the fail returns `beta` (fail-hard),
   never an unproven mate score.
7. **Move ordering** — see below.
8. **The move loop**: PVS with LMR (details below), collecting history/killer/countermove
   statistics on beta cutoffs.
9. **TT store** — skipped entirely if the node was aborted, so a half-searched result can never
   poison later searches.

### Move ordering (`src/search/ordering.rs`)

Alpha-beta strength is mostly ordering. `order_full` scores moves into strict bands:

| Band | Score | Source |
|---|---|---|
| TT move | 1,000,000 | previous search of this node |
| Captures | 10,000 + MVV-LVA | victim value ×100 − attacker value |
| Promotions | 9,000 | |
| Killer 1 / 2 | 8,000 / 7,500 | quiets that cut at this ply |
| **Countermove** | 7,250 | quiet that last refuted the opponent's previous move |
| Quiets | ≤ 6,900 | butterfly history (±6,500) + **raptor bonus** (+400) |

- **Countermove table** — indexed `[prev.from × 144 + prev.to]`, it remembers the refutation of
  each opponent move. A refutation tends to stay a refutation wherever that move appears in the
  tree, and unlike history it adapts instantly (single overwrite, no decay needed).
- **Raptor bonus** — HyperChess-specific: a quiet Eagle/Hawk move landing within Chebyshev
  distance 4 (their jump range) of the enemy king enters *strike range*, from which its check
  cannot be blocked. Trying these regrouping moves early finds king attacks far sooner than
  history statistics would on a 144-square board. The bonus is small (+400) and stays inside the
  quiet band, so proven statistics (killers, countermoves) always outrank the heuristic.
- **History gravity** — the Stockfish formula `h += bonus − |h|·bonus/MAX` keeps the butterfly
  table in ±16,384 with graceful decay; failed quiets are penalised symmetrically.

### PVS + LMR (the move loop)

- The first (best-ordered) move is searched with the full window — it is the PV candidate.
- Every later move gets a **null-window probe** (`alpha, alpha+1`); only a probe that lands
  *inside* the window earns a full re-search, so the final score of the chosen move is always
  exact. Probes ≥ beta are already cutoffs.
- **Late-move reductions**: quiet, non-killer moves late in the ordering are probed at reduced
  depth first (1–3 plies, by depth and move count). A reduced probe that beats alpha is
  re-searched at full depth — LMR can only save time, never change the result.
- **Frontier futility** *(Aggressive profile)*: at depth ≤ 2, quiet moves that cannot lift a
  hopeless static eval past alpha are skipped (never the first move, so a bound always exists).

### Quiescence (`src/search/quiescence.rs`)

At depth 0 the search does not stop — it plays out captures until the position is quiet, killing
the horizon effect. Two correctness rules:

- **No stand-pat in check.** A checked side must respond; all evasions are searched, and no
  evasion means checkmate — scored by distance from the root, not by static eval.
- **The budget applies here too.** Every quiescence node ticks the same counter, so node limits
  and deadlines cannot be overshot by a capture cascade.

Captures losing material on the exchange are skipped via **SEE** (below); the Aggressive profile
adds **delta pruning** (skip captures that cannot bring stand-pat near alpha even winning the
victim outright — promotions exempt).

### Static Exchange Evaluation (`src/search/see.rs`)

SEE plays out the full capture/recapture sequence on a square — least valuable attacker first,
either side free to stop — and returns the net material outcome. X-rays come free because
attackers are recomputed from current occupancy; en passant resolves the victim *behind* the
destination; a king may only recapture when nothing defends the square. Raptor values (Eagle 700,
Hawk 550) sit between rook and queen/bishop, reflecting their forking power on the big board.

## Profiles

| | Balanced | Strategic | Aggressive |
|---|---|---|---|
| TT entries | 2²⁰ | 2²² | 2²² |
| Root ordering | static bands | full eval-guided (depth ≥ 3) | static bands |
| LMR threshold | move 3 | move 4 (more conservative) | move 3 |
| Speculative pruning¹ | — | — | ✓ |

¹ Reverse futility + frontier futility + quiescence delta pruning: the classic
strength-for-tactical-risk trade every top engine makes. PVS and aspiration windows are always on
— they are pure speedups.

## MCTS (`src/mcts.rs`)

A complete **UCT** searcher, useful for analysis diversity and as the integration point for
GPU/NN evaluation:

- Arena-allocated tree (indices, not pointers) with `UCB1` selection (`C = √2`), expansion,
  **static-eval rollouts** (far stronger than random playouts on a 144-square board; values
  clamped to ±3000 and normalised to [-1, 1]), and sign-alternating backpropagation.
- Terminal nodes score exactly (mate/draw), and are never expanded.
- **Virtual loss** supports batched leaf evaluation: `mcts_with_eval_bounded` collects a batch of
  leaves, lets a caller-supplied evaluator (e.g. the CUDA crate) score them all at once, and
  falls back to CPU evaluation for any leaf the batch missed.
- The same three stop conditions as the alpha-beta driver apply (`mcts_bounded`).

## Safety invariants (the checklist)

These are the properties the test suite pins down; PRs must preserve them:

1. A legal move is **always** returned when one exists — fallback chain: deepest completed
   iteration → best root move of the aborted iteration → first generated move.
2. No TT cutoff at the root; no TT store from an aborted node.
3. Mate scores are root-relative everywhere (search, quiescence, TT round-trip).
4. Null-move never returns an unproven mate; never runs in check/zugzwang.
5. LMR/PVS re-search rules guarantee the chosen move's score is exact.
6. Quiescence in check searches evasions (no stand-pat) and honours the node budget.
7. The 112-move and insufficient-material draws never silence the root.
8. Pruning (reverse/frontier futility, delta) is disabled near mate bounds and in check —
   a forced mate can never be pruned away (`pro_profile_finds_mate_in_one` test).

## Testing

- **Golden perft** values pin move generation; `searcher_unification_golden` pins that every
  public searcher name reaches the canonical search.
- Unit tests cover each heuristic in isolation (ordering bands, SEE exchanges incl. en passant,
  quiescence mate/evasion/abort, countermove and raptor ordering).
- Integration tests replay full games and assert the board is never corrupted.
- `examples/golden_measure.rs` and `examples/node_cap_probe.rs` measure strength/cost trade-offs
  when tuning constants.

## Tuning guide

The interesting knobs, all in `src/timed.rs` / `src/search/ordering.rs`:

| Constant | Value | Effect of raising |
|---|---|---|
| aspiration `delta` | 50 cp | fewer re-searches, weaker ordering pressure |
| `LMR_MIN_DEPTH` / `LMR_MIN_MOVE` | 3 / 3 | less reduction → slower, safer |
| null-move `R` | 2–3 | deeper verification → slower, safer |
| reverse futility margin | 120·depth | more pruning → faster, riskier |
| frontier futility margin | 150·depth | more pruning → faster, riskier |
| raptor bonus | +400 | stronger raptor bias in quiet ordering |
| countermove band | 7,250 | must stay between killer 2 and the quiet cap |

Benchmark any change with fixed-node self-play (`SearchLimits::nodes`) so results are
reproducible; wall-clock runs vary with machine load.
