// SPDX-License-Identifier: GPL-3.0-or-later
// HyperChess Core — hyperchess-driver
// File: crates/hyperchess-driver/src/cli/mod.rs
// Version: 1.0.0
// Copyright (c) 2026 HyperChess Developer Team

//! Engine-vs-engine game loop, PNG board rendering, and game export (HFEN-I/
//! HPGN-I/JSON) — the `hyperchess play` / `perft` / `show` / `gpu-info`
//! subcommands' implementation.

pub mod export;
pub mod game;
pub mod render;
