# hyperchess-search

Search algorithms for HyperChess — alpha-beta, iterative deepening, MCTS, and the anytime
`TimedSearcher` used by the interactive (WASM + server) paths. Depends on `hyperchess-rules` for
board/move types; nothing in `hyperchess-rules` depends back on this crate.

Every public searcher name delegates to the single canonical `TimedSearcher`: iterative
deepening with aspiration windows, PVS, a transposition table with root-safe probing, check
extensions, null-move pruning, LMR, SEE-pruned quiescence, and an ordering stack of TT move →
MVV-LVA captures → killers → **countermove table** → butterfly history with the
HyperChess-specific **raptor bonus** (Eagle/Hawk moves into the enemy king's strike zone are
tried early — their jump checks cannot be blocked by interposition). The `Aggressive` profile
adds reverse futility, frontier futility, and quiescence delta pruning.

**The full technique-by-technique guide — including the safety invariants every PR must
preserve and a tuning table — is [`docs/search-architecture.md`](../../../docs/search-architecture.md).**

Full architecture, extraction history, and roadmap: see the workspace-level
[`docs/hyperchess-core-extraction-plan.md`](../../../docs/hyperchess-core-extraction-plan.md)
(§2 component table, §12 Phase 3 — including the five files carried over here from
`hyperchess-rules`' Phase 1 deferral: `tests/regression.rs`, `tests/rules_integration.rs`,
`examples/{golden_measure,node_cap_probe}.rs`).

## Zero dependencies

The default build has **no external crates** — the dependency tree is exactly
`hyperchess-search → hyperchess-rules → hyperchess-eval`. Randomness (RandomBot, the MCTS
expansion shuffle) uses the rules crate's own XorShift64* PRNG, seeded from the position's
Zobrist key, which also makes MCTS searches exactly reproducible.

## `wasm` feature

Off by default. Enables `js_sys::Date::now()` for `timed::clock` (browsers can't interrupt a
synchronous WASM call, so the search itself watches the wall clock) — `js-sys` is the single
crate this feature adds. It still forwards to `hyperchess-rules`' `wasm` feature, which is now
an intentional no-op kept for compatibility (it used to pull getrandom's "js" feature back when
`rand` was a runtime dependency). `hyperchess-wasm` (Phase 6) depends on this crate with the
feature enabled.
