// SPDX-License-Identifier: GPL-3.0-or-later
// HyperChess Core — hyperchess-search
// File: crates/hyperchess-search/src/iterative.rs
// Version: 1.0.0
// Copyright (c) 2026 HyperChess Developer Team

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

/// Iterative-deepening search to a fixed maximum depth.
///
/// Shares the [`TimedSearcher`] core with [`crate::AlphaBetaSearcher`]; the
/// canonical search already deepens iteratively, so this exists as a distinct
/// name for CLI/UCI algorithm selection rather than as a different algorithm.
pub struct IterativeSearcher {
    inner: TimedSearcher,
}

impl IterativeSearcher {
    /// A searcher with an empty transposition table.
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
