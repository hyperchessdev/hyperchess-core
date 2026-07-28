// SPDX-License-Identifier: GPL-3.0-or-later
// HyperChess Core — hyperchess-rules
// File: crates/hyperchess-rules/src/core/sq.rs
// Version: 1.0.0
// Copyright (c) 2026 HyperChess Developer Team

//! Square type for the 12x12 board (0-143).

use super::bitboard::BitBoard;
use super::masks::*;
use std::fmt;

/// A square on the 12x12 board, represented as a u8 (0-143).
/// `sq = rank * 12 + file` where a1=0, l1=11, a2=12, ..., l12=143.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct SQ(pub u8);

/// Sentinel value for "no square".
pub const NO_SQ: SQ = SQ(144);

impl SQ {
    /// Creates a SQ from file and rank indices (0-based).
    #[inline(always)]
    pub const fn make(file: u8, rank: u8) -> SQ {
        SQ(rank * 12 + file)
    }

    /// Returns the file index (0-11).
    #[inline(always)]
    pub const fn file_idx(self) -> u8 {
        self.0 % 12
    }

    /// Returns the rank index (0-11).
    #[inline(always)]
    pub const fn rank_idx(self) -> u8 {
        self.0 / 12
    }

    /// Returns true if the square index is valid (0-143).
    #[inline(always)]
    pub const fn is_okay(self) -> bool {
        self.0 < 144
    }

    /// Returns the File enum.
    #[inline(always)]
    pub fn file(self) -> super::File {
        super::File::from_index(self.file_idx())
    }

    /// Returns the Rank enum.
    #[inline(always)]
    pub fn rank(self) -> super::Rank {
        super::Rank::from_index(self.rank_idx())
    }

    /// Converts this square to a single-bit BitBoard.
    #[inline(always)]
    pub fn to_bb(self) -> BitBoard {
        BitBoard::from_sq(self)
    }

    /// Returns the string representation (e.g., "a1", "l12").
    pub fn notation(self) -> String {
        if !self.is_okay() {
            return String::from("--");
        }
        let file = FILE_DISPLAYS[self.file_idx() as usize];
        let rank = self.rank_idx() + 1;
        format!("{}{}", file, rank)
    }

    /// Computes distance (Chebyshev) between two squares.
    pub fn distance(self, other: SQ) -> u8 {
        let file_dist = if self.file_idx() > other.file_idx() {
            self.file_idx() - other.file_idx()
        } else {
            other.file_idx() - self.file_idx()
        };
        let rank_dist = if self.rank_idx() > other.rank_idx() {
            self.rank_idx() - other.rank_idx()
        } else {
            other.rank_idx() - self.rank_idx()
        };
        file_dist.max(rank_dist)
    }

    /// Applies a direction offset, returning None if the result is off-board.
    #[inline]
    pub fn offset(self, dir: i16) -> Option<SQ> {
        let new_sq = self.0 as i16 + dir;
        if !(0..144).contains(&new_sq) {
            return None;
        }
        let new_sq = new_sq as u8;
        // Check for wrap-around (file distance should be at most 2 for any valid move)
        let old_file = self.file_idx() as i16;
        let new_file = (new_sq % 12) as i16;
        let file_diff = (old_file - new_file).abs();
        if file_diff > 2 {
            return None;
        }
        Some(SQ(new_sq))
    }

    /// Applies a direction offset without wrap checking (for ray-based moves where
    /// the caller already ensures valid direction stepping).
    #[inline]
    pub fn offset_unchecked(self, dir: i16) -> Option<SQ> {
        let new_sq = self.0 as i16 + dir;
        if !(0..144).contains(&new_sq) {
            return None;
        }
        let new_sq_u8 = new_sq as u8;
        // For orthogonal moves E/W, check file wrap
        // For all moves, check that the file distance is reasonable
        let old_file = self.file_idx() as i16;
        let new_file = (new_sq_u8 % 12) as i16;
        let file_diff = (old_file - new_file).abs();
        // For knight offsets, file_diff can be up to 2
        // For single-step directions, file_diff is at most 1
        // We allow up to 2 for knight compatibility
        if file_diff > 2 {
            return None;
        }
        Some(SQ(new_sq_u8))
    }
}

impl fmt::Display for SQ {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.notation())
    }
}

impl fmt::Debug for SQ {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "SQ({}={})", self.0, self.notation())
    }
}

impl std::ops::BitXor for SQ {
    type Output = SQ;
    #[inline(always)]
    fn bitxor(self, rhs: SQ) -> SQ {
        SQ(self.0 ^ rhs.0)
    }
}

// Named square constants for convenience
impl SQ {
    /// Bottom-left corner, index 0.
    pub const A1: SQ = SQ::make(0, 0);
    /// b1.
    pub const B1: SQ = SQ::make(1, 0);
    /// c1.
    pub const C1: SQ = SQ::make(2, 0);
    /// Bottom-right corner — the l-file is the 12th, not h as in chess.
    pub const L1: SQ = SQ::make(11, 0);
    /// a2.
    pub const A2: SQ = SQ::make(0, 1);
    /// White's king start. Rank 2, not rank 1: the back rank on a 12x12 board
    /// carries the extra pieces, and the royal rank sits one row in.
    pub const G2: SQ = SQ::make(6, 1);
    /// Top-left corner.
    pub const A12: SQ = SQ::make(0, 11);
    /// Top-right corner, index 143.
    pub const L12: SQ = SQ::make(11, 11);
    /// Black's king start, mirroring [`SQ::G2`].
    pub const G11: SQ = SQ::make(6, 10);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_make_file_rank() {
        let sq = SQ::make(0, 0); // a1
        assert_eq!(sq.0, 0);
        assert_eq!(sq.file_idx(), 0);
        assert_eq!(sq.rank_idx(), 0);

        let sq = SQ::make(11, 0); // l1
        assert_eq!(sq.0, 11);
        assert_eq!(sq.file_idx(), 11);
        assert_eq!(sq.rank_idx(), 0);

        let sq = SQ::make(0, 1); // a2
        assert_eq!(sq.0, 12);
        assert_eq!(sq.file_idx(), 0);
        assert_eq!(sq.rank_idx(), 1);

        let sq = SQ::make(11, 11); // l12
        assert_eq!(sq.0, 143);
        assert_eq!(sq.file_idx(), 11);
        assert_eq!(sq.rank_idx(), 11);
    }

    #[test]
    fn test_is_okay() {
        assert!(SQ(0).is_okay());
        assert!(SQ(143).is_okay());
        assert!(!SQ(144).is_okay());
        assert!(!SQ(255).is_okay());
    }

    #[test]
    fn test_to_string() {
        assert_eq!(SQ::make(0, 0).notation(), "a1");
        assert_eq!(SQ::make(11, 0).notation(), "l1");
        assert_eq!(SQ::make(0, 11).notation(), "a12");
        assert_eq!(SQ::make(6, 1).notation(), "g2");
    }

    #[test]
    fn test_distance() {
        let a1 = SQ::make(0, 0);
        let l12 = SQ::make(11, 11);
        assert_eq!(a1.distance(l12), 11);
        assert_eq!(a1.distance(a1), 0);
    }

    #[test]
    fn test_offset() {
        // a1 north = a2
        assert_eq!(SQ(0).offset(12), Some(SQ(12)));
        // a1 south = None (off board)
        assert_eq!(SQ(0).offset(-12), None);
        // l1 east = None (wraps)
        assert_eq!(SQ(11).offset(1), None);
        // a1 west = None (wraps)
        assert_eq!(SQ(0).offset(-1), None);
    }

    #[test]
    fn test_king_start_squares() {
        assert_eq!(SQ::G2.0, 18); // White king
        assert_eq!(SQ::G11.0, 126); // Black king
    }
}
