//! `IterativeSearcher` — backwards-compatible name for the canonical search.
//!
//! The canonical [`TimedSearcher`] *is* an iterative-deepening alpha-beta with a
//! transposition table, killers/history, check extensions, LMR, null-move pruning
//! and SEE-pruned quiescence. This type is a thin wrapper that holds and reuses one
//! so every caller shares the same implementation.

use crate::TimedSearcher;
use hyperchess_rules::board::Board;
use hyperchess_rules::core::piece_move::HyperMove;
use hyperchess_rules::tools::Searcher;

pub struct IterativeSearcher {
    inner: TimedSearcher,
}

impl IterativeSearcher {
    pub fn new() -> Self {
        Self {
            inner: TimedSearcher::new(),
        }
    }
}

impl Default for IterativeSearcher {
    fn default() -> Self {
        Self::new()
    }
}

impl Searcher for IterativeSearcher {
    fn best_move(&mut self, board: &Board, max_depth: u32) -> HyperMove {
        self.inner.best_move(board, max_depth)
    }

    fn name(&self) -> &str {
        "IterativeDeepening"
    }
}
