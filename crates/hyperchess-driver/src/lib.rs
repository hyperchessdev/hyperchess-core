// SPDX-License-Identifier: GPL-3.0-or-later
// HyperChess Core — hyperchess-driver
// File: crates/hyperchess-driver/src/lib.rs
// Version: 1.0.0
// Copyright (c) 2026 HyperChess Developer Team

//! HyperChess driver — the integration layer between the engine
//! (`hyperchess-rules` + `hyperchess-search`[`-cuda`]) and the outside world.
//!
//! Three independent integration paths, all exposed as subcommands of the
//! `hyperchess` binary (see `main.rs`):
//! - [`cli`] — engine-vs-engine games, PNG board rendering, HFEN-I/HPGN-I/JSON export.
//! - [`uci`] — the UCI protocol server, client, and connection pool.
//! - [`api`] — a stateless REST/OpenAPI service (no DB, no required env vars
//!   to boot) — see docs/hyperchess-core-extraction-plan.md §6/§12 Phase 5.

pub mod api;
pub mod cli;
pub mod uci;
