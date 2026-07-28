# hyperchess-search-cuda

GPU (CUDA) accelerated search: batch position evaluation, CUDA MCTS, and GPU-guided alpha-beta.
Depends on both `hyperchess-rules` and `hyperchess-search`.

**`publish = false` — never goes to crates.io.** The `cuda` feature's `cust`/`cust_raw`/
`cuda_builder` dependencies point at a local, unversioned `rust-cuda` checkout
(`/projects/github/rust-cuda`), which crates.io rejects outright. This is a known, accepted
limitation, not a bug — see the workspace-level
[`docs/hyperchess-core-extraction-plan.md`](../../../docs/hyperchess-core-extraction-plan.md) §5.
CPU search (`hyperchess-search`) is the crates.io/npm-distributed default; this crate is a
source-only, opt-in accelerator for anyone running the engine locally with a GPU and that
checkout.

Default build (`cargo build -p hyperchess-search-cuda`, no `cuda` feature) succeeds trivially —
the crate is empty without it, matching the source repo's exact module-gating (`cuda_backend.rs`
etc. unconditionally `use cust::...` internally, so they can only compile once `cust` is actually
present as a dependency). `--features cuda` requires the local `rust-cuda` checkout and a
CUDA-capable GPU/driver.

The `kernels/` subdirectory is a separate, standalone Cargo project (its own `[workspace]`,
compiled to PTX via `build.rs` + `cuda_builder`, not a normal linked dependency) — this mirrors
the source repo exactly.
