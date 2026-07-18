//! HyperChess driver — the integration layer between the engine
//! (`hyperchess-rules` + `hyperchess-search`[`-cuda`]) and the outside world.
//!
//! Two independent integration paths, both exposed as subcommands of the
//! `hyperchess` binary (see `main.rs`):
//! - [`cli`] — engine-vs-engine games, PNG board rendering, HFEN-I/HPGN-I/JSON export.
//! - [`uci`] — the UCI protocol server, client, and connection pool.
//!
//! The (planned, not yet built — see
//! docs/hyperchess-core-extraction-plan.md §6/§12 Phase 5) stateless REST/OpenAPI
//! `api` module will live here too, as a third subcommand.

pub mod cli;
pub mod uci;
