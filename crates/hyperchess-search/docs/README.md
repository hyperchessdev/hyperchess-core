# hyperchess-search

Search algorithms for HyperChess — alpha-beta, iterative deepening, MCTS, and the anytime
`TimedSearcher` used by the interactive (WASM + server) paths. Depends on `hyperchess-rules` for
board/move types; nothing in `hyperchess-rules` depends back on this crate.

Full architecture, extraction history, and roadmap: see the workspace-level
[`docs/hyperchess-core-extraction-plan.md`](../../../docs/hyperchess-core-extraction-plan.md)
(§2 component table, §12 Phase 3 — including the five files carried over here from
`hyperchess-rules`' Phase 1 deferral: `tests/regression.rs`, `tests/rules_integration.rs`,
`examples/{golden_measure,node_cap_probe}.rs`).

## `wasm` feature

Off by default. Enables `js_sys::Date::now()` for `timed::clock` (browsers can't interrupt a
synchronous WASM call, so the search itself watches the wall clock) and forwards to
`hyperchess-rules`' own `wasm` feature (needed for `rand`'s `getrandom` on `wasm32-unknown-unknown`).
`hyperchess-wasm` (Phase 6) depends on this crate with the feature enabled.
