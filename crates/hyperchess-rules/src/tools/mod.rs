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
