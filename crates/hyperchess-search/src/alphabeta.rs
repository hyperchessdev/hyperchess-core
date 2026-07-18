//! `AlphaBetaSearcher` — backwards-compatible name for the canonical search.
//!
//! Historically this was a separate fixed-depth negamax. The engine now has a
//! single canonical search ([`TimedSearcher`]) carrying every improvement
//! (transposition table, killers/history ordering, check extensions, late-move
//! reductions, null-move pruning and SEE-pruned quiescence). To guarantee that
//! *every* caller — CLI, WASM, server — gets the same strong search, this type is
//! a thin wrapper that holds and reuses a [`TimedSearcher`].

use crate::TimedSearcher;
use hyperchess_rules::board::Board;
use hyperchess_rules::core::piece_move::HyperMove;
use hyperchess_rules::tools::Searcher;

pub struct AlphaBetaSearcher {
    inner: TimedSearcher,
}

impl AlphaBetaSearcher {
    pub fn new() -> Self {
        Self {
            inner: TimedSearcher::new(),
        }
    }
}

impl Default for AlphaBetaSearcher {
    fn default() -> Self {
        Self::new()
    }
}

impl Searcher for AlphaBetaSearcher {
    fn best_move(&mut self, board: &Board, depth: u32) -> HyperMove {
        self.inner.best_move(board, depth)
    }

    fn name(&self) -> &str {
        "AlphaBeta"
    }
}
