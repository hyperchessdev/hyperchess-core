// SPDX-License-Identifier: GPL-3.0-or-later
// HyperChess Core — hyperchess-search
// File: crates/hyperchess-search/src/pro.rs
// Version: 1.0.0
// Copyright (c) 2026 HyperChess Developer Team

//! `AggressiveSearcher` — the "commercial-grade" search option.
//!
//! The canonical [`TimedSearcher`] running the [`SearchProfile::Aggressive`] profile:
//! the full technique set of top commercial engines (strategic/Stockfish class).
//! On top of the always-on machinery (iterative deepening, transposition table,
//! killers/history ordering, check extensions, LMR, null-move pruning, PVS,
//! aspiration windows, SEE- and evasion-aware quiescence) it enables the
//! speculative pruning family:
//!
//! * **Reverse futility** — shallow nodes whose static eval is far above beta
//!   fail high without a search.
//! * **Frontier futility** — quiet moves at depth ≤ 2 that cannot lift a
//!   hopeless static eval past alpha are skipped.
//! * **Quiescence delta pruning** — captures that cannot bring stand-pat near
//!   alpha even when winning the victim outright are skipped.
//!
//! The pruning trades a small tactical risk at frontier nodes for a much deeper
//! effective search — the same trade every top engine makes.

use crate::TimedSearcher;
use hyperchess_rules::board::Board;
use hyperchess_rules::core::piece_move::HyperMove;
use hyperchess_rules::tools::Searcher;

/// Holds and reuses a Pro-profile canonical search across moves.
pub struct AggressiveSearcher {
    inner: TimedSearcher,
}

impl AggressiveSearcher {
    /// An aggressive-profile searcher with an empty transposition table.
    pub fn new() -> Self {
        Self {
            inner: TimedSearcher::pro(),
        }
    }
}

impl Default for AggressiveSearcher {
    fn default() -> Self {
        Self::new()
    }
}

impl Searcher for AggressiveSearcher {
    fn best_move(&mut self, board: &Board, max_depth: u32) -> HyperMove {
        self.inner.best_move(board, max_depth)
    }

    fn name(&self) -> &str {
        "Aggressive"
    }
}
