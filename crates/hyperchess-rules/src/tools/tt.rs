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
    pub key: u64,
    pub best_move: HyperMove,
    pub score: Value,
    pub depth: i32,
    pub flag: TTFlag,
}

impl TTEntry {
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
