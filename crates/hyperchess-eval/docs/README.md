# hyperchess-eval

no_std single-source evaluation math shared by the CPU evaluator (`hyperchess-search`, via
`hyperchess-rules::tools::eval`) and the GPU kernel (`hyperchess-search-cuda`, Phase 4).

Full architecture, extraction history, and roadmap: see the workspace-level
[`docs/hyperchess-core-extraction-plan.md`](../../../docs/hyperchess-core-extraction-plan.md)
(§2 component table, §12 Phase 2). This crate's own `docs/` holds anything specific to
`hyperchess-eval` itself as it grows past a single `lib.rs`.
