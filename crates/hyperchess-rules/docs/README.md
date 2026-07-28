# hyperchess-rules

Board representation, move generation, and legality for HyperChess — a 12×12 chess variant with
Eagle and Hawk pieces. No search here; see `hyperchess-search` (Phase 3), which depends on this
crate, not the other way around.

Full architecture, extraction history, and roadmap: see the workspace-level
[`docs/hyperchess-core-extraction-plan.md`](../../../docs/hyperchess-core-extraction-plan.md)
(§1 for what changed vs. the source repo, §2 component table, §12 Phase 1 for exactly what was
included/excluded and why — including the five bots-dependent test/example files deferred to
Phase 3).
