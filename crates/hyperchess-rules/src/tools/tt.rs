// SPDX-License-Identifier: GPL-3.0-or-later
// HyperChess Core — hyperchess-rules
// File: crates/hyperchess-rules/src/tools/tt.rs
// Version: 1.0.0
// Copyright (c) 2026 HyperChess Developer Team

//! Transposition table.

use crate::core::piece_move::HyperMove;
use crate::core::score::Value;

/// Transposition table entry type.
#[derive(Copy, Clone, PartialEq, Eq)]
pub enum TTFlag {
    Exact,
    LowerBound,
    UpperBound,
}

/// A single transposition table entry.
#[derive(Copy, Clone)]
pub struct TTEntry {
    /// Full Zobrist hash of the position. Stored in full even though the index
    /// only uses the low bits, so a slot collision can be detected rather than
    /// silently returning another position's score.
    pub key: u64,
    /// Best move found at this node, used for move ordering even when the
    /// stored score itself is unusable at the current depth.
    pub best_move: HyperMove,
    /// Score, interpreted according to `flag`.
    pub score: Value,
    /// Remaining depth this entry was searched to; a probe may only trust the
    /// score if its own remaining depth is no greater.
    pub depth: i32,
    /// Whether `score` is exact or only a bound.
    pub flag: TTFlag,
}

impl TTEntry {
    /// A slot that no probe can match.
    ///
    /// `depth: -1` is what makes it unusable: every real search stores a
    /// non-negative depth, so the depth check rejects an empty slot before the
    /// key is ever trusted.
    pub fn empty() -> Self {
        TTEntry {
            key: 0,
            best_move: HyperMove::null(),
            score: 0,
            depth: -1,
            flag: TTFlag::Exact,
        }
    }
}

/// The transposition table.
pub struct TranspositionTable {
    entries: Vec<TTEntry>,
    mask: usize,
}

impl TranspositionTable {
    /// Creates a new TT with the given number of entries (rounded to power of 2).
    pub fn new(size: usize) -> Self {
        let size = size.next_power_of_two();
        TranspositionTable {
            entries: vec![TTEntry::empty(); size],
            mask: size - 1,
        }
    }

    /// Probes the TT for an entry matching the given key.
    pub fn probe(&self, key: u64) -> Option<&TTEntry> {
        let idx = (key as usize) & self.mask;
        let entry = &self.entries[idx];
        if entry.key == key && entry.depth >= 0 {
            Some(entry)
        } else {
            None
        }
    }

    /// Stores an entry in the TT.
    pub fn store(
        &mut self,
        key: u64,
        best_move: HyperMove,
        score: Value,
        depth: i32,
        flag: TTFlag,
    ) {
        let idx = (key as usize) & self.mask;
        let entry = &mut self.entries[idx];
        // Always replace (simplest replacement scheme)
        if depth >= entry.depth || entry.key != key {
            *entry = TTEntry {
                key,
                best_move,
                score,
                depth,
                flag,
            };
        }
    }

    /// Clears the TT.
    pub fn clear(&mut self) {
        for entry in self.entries.iter_mut() {
            *entry = TTEntry::empty();
        }
    }
}
