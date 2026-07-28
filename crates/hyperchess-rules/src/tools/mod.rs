// SPDX-License-Identifier: GPL-3.0-or-later
// HyperChess Core — hyperchess-rules
// File: crates/hyperchess-rules/src/tools/mod.rs
// Version: 1.0.0
// Copyright (c) 2026 HyperChess Developer Team

//! Search utilities: evaluation, transposition table, PRNG.

pub mod eval;
pub mod prng;
pub mod tt;

use crate::board::Board;
use crate::core::piece_move::HyperMove;

/// Trait for search algorithms.
pub trait Searcher {
    /// Returns the best move for the current position.
    fn best_move(&mut self, board: &Board, depth: u32) -> HyperMove;

    /// Returns the name of this searcher.
    fn name(&self) -> &str;
}
