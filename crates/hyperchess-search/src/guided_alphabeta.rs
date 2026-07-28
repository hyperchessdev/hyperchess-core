// SPDX-License-Identifier: GPL-3.0-or-later
// HyperChess Core — hyperchess-search
// File: crates/hyperchess-search/src/guided_alphabeta.rs
// Version: 1.0.0
// Copyright (c) 2026 HyperChess Developer Team

//! `GuidedAlphaBeta` / `GuidedIterative` — backwards-compatible names for the
//! canonical search.
//!
//! These names all use the canonical [`TimedSearcher`]. `GuidedAlphaBeta` and
//! `GuidedIterative` are compatibility aliases. `StrategicSearcher` enables a
//! stronger tactical profile: larger TT, root child static-eval ordering, and a more
//! conservative LMR threshold.

use crate::TimedSearcher;
use hyperchess_rules::board::Board;
use hyperchess_rules::core::piece_move::HyperMove;
use hyperchess_rules::tools::Searcher;

/// Holds and reuses a canonical search across moves (like [`GuidedIterative`]),
/// so the transposition table survives between `best_move` calls instead of
/// being rebuilt from scratch every move.
pub struct GuidedAlphaBeta {
    inner: TimedSearcher,
}

impl GuidedAlphaBeta {
    /// A searcher with an empty transposition table.
    pub fn new() -> Self {
        Self {
            inner: TimedSearcher::new(),
        }
    }
}

impl Default for GuidedAlphaBeta {
    fn default() -> Self {
        Self::new()
    }
}

impl Searcher for GuidedAlphaBeta {
    fn best_move(&mut self, board: &Board, depth: u32) -> HyperMove {
        self.inner.best_move(board, depth)
    }
    fn name(&self) -> &str {
        "GuidedAlphaBeta"
    }
}

/// Holds and reuses a canonical search across moves.
pub struct GuidedIterative {
    inner: TimedSearcher,
}

impl GuidedIterative {
    /// A searcher with an empty transposition table.
    pub fn new() -> Self {
        Self {
            inner: TimedSearcher::new(),
        }
    }
}

impl Default for GuidedIterative {
    fn default() -> Self {
        Self::new()
    }
}

impl Searcher for GuidedIterative {
    fn best_move(&mut self, board: &Board, max_depth: u32) -> HyperMove {
        self.inner.best_move(board, max_depth)
    }
    fn name(&self) -> &str {
        "GuidedIterative"
    }
}

/// Search on the [`SearchProfile::Strategic`] profile: later late-move
/// reductions and guided root ordering, trading nodes for positional accuracy.
///
/// [`SearchProfile::Strategic`]: crate::SearchProfile::Strategic
pub struct StrategicSearcher {
    inner: TimedSearcher,
}

impl StrategicSearcher {
    /// A strategic-profile searcher with an empty transposition table.
    pub fn new() -> Self {
        Self {
            inner: TimedSearcher::strategic(),
        }
    }
}

impl Default for StrategicSearcher {
    fn default() -> Self {
        Self::new()
    }
}

impl Searcher for StrategicSearcher {
    fn best_move(&mut self, board: &Board, max_depth: u32) -> HyperMove {
        self.inner.best_move(board, max_depth)
    }

    fn name(&self) -> &str {
        "Strategic"
    }
}
