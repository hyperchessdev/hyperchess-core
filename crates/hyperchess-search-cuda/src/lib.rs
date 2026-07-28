//! GPU (CUDA) accelerated search — batch position evaluation, CUDA MCTS, and
//! GPU-guided alpha-beta. Depends on both `hyperchess-rules` and
//! `hyperchess-search` (GPU batch eval augments, not replaces, CPU search
//! internals for hybrid CPU/GPU computation).
//!
//! **Never published to crates.io** (`publish = false`) — the `cuda` feature's
//! `cust`/`cust_raw`/`cuda_builder` dependencies point at a local, unversioned
//! `rust-cuda` checkout, which crates.io rejects. Source-only, opt-in for
//! anyone building locally with a GPU and that checkout — see
//! docs/hyperchess-core-extraction-plan.md §5.
//!
//! Mirrors the source repo's exact module-gating: without the `cuda` feature
//! (the default), this crate is empty — the modules themselves unconditionally
//! `use cust::...` internally, so they can only compile once `cust` is
//! actually present as a dependency (i.e. once the feature enables it).

#[cfg(feature = "cuda")]
pub mod cuda_backend;

#[cfg(feature = "cuda")]
pub mod cuda_mcts;

#[cfg(feature = "cuda")]
pub mod gpu_alphabeta;
