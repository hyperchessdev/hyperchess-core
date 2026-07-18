//! MoveList — fixed-size array of moves.

use super::masks::MAX_MOVES;
use super::piece_move::{HyperMove, ScoringMove};

/// A list of moves stored in a fixed-size array.
pub struct MoveList {
    inner: [HyperMove; MAX_MOVES],
    len: usize,
}

impl MoveList {
    /// Creates an empty MoveList.
    #[inline]
    pub fn new() -> Self {
        MoveList {
            inner: [HyperMove::null(); MAX_MOVES],
            len: 0,
        }
    }

    /// Pushes a move onto the list.
    #[inline(always)]
    pub fn push(&mut self, m: HyperMove) {
        debug_assert!(self.len < MAX_MOVES);
        self.inner[self.len] = m;
        self.len += 1;
    }

    /// Returns the number of moves.
    #[inline(always)]
    pub fn len(&self) -> usize {
        self.len
    }

    /// Returns true if empty.
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Returns a slice of all moves.
    #[inline(always)]
    pub fn as_slice(&self) -> &[HyperMove] {
        &self.inner[..self.len]
    }

    /// Returns a mutable slice.
    #[inline(always)]
    pub fn as_mut_slice(&mut self) -> &mut [HyperMove] {
        &mut self.inner[..self.len]
    }

    /// Returns an iterator over the moves.
    #[inline]
    pub fn iter(&self) -> std::slice::Iter<'_, HyperMove> {
        self.as_slice().iter()
    }

    /// Gets a move by index.
    #[inline(always)]
    pub fn get(&self, idx: usize) -> HyperMove {
        self.inner[idx]
    }

    /// Checks if the list contains a specific move.
    pub fn contains(&self, m: HyperMove) -> bool {
        self.as_slice().contains(&m)
    }
}

impl Default for MoveList {
    fn default() -> Self {
        MoveList::new()
    }
}

impl std::fmt::Debug for MoveList {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "MoveList[")?;
        for (i, m) in self.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{}", m)?;
        }
        write!(f, "]")
    }
}

/// A list of scored moves for move ordering.
pub struct ScoringMoveList {
    inner: [ScoringMove; MAX_MOVES],
    len: usize,
}

impl ScoringMoveList {
    pub fn new() -> Self {
        ScoringMoveList {
            inner: [ScoringMove::null(); MAX_MOVES],
            len: 0,
        }
    }

    pub fn push(&mut self, m: ScoringMove) {
        debug_assert!(self.len < MAX_MOVES);
        self.inner[self.len] = m;
        self.len += 1;
    }

    pub fn push_move(&mut self, m: HyperMove) {
        self.push(ScoringMove::new(m));
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn as_slice(&self) -> &[ScoringMove] {
        &self.inner[..self.len]
    }

    pub fn as_mut_slice(&mut self) -> &mut [ScoringMove] {
        &mut self.inner[..self.len]
    }

    /// Sort moves by score (highest first) for move ordering.
    pub fn sort(&mut self) {
        self.as_mut_slice()
            .sort_unstable_by_key(|m| std::cmp::Reverse(m.score));
    }
}

impl Default for ScoringMoveList {
    fn default() -> Self {
        ScoringMoveList::new()
    }
}
