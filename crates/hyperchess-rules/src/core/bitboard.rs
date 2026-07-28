// SPDX-License-Identifier: GPL-3.0-or-later
// HyperChess Core — hyperchess-rules
// File: crates/hyperchess-rules/src/core/bitboard.rs
// Version: 1.0.0
// Copyright (c) 2026 HyperChess Developer Team

//! BitBoard type for the 12x12 HyperChess board.
//!
//! A `BitBoard` is represented as `[u64; 3]` (192 bits), with 144 bits used.
//! - word[0]: squares 0-63
//! - word[1]: squares 64-127
//! - word[2]: squares 128-143 (16 bits used)
//!
//! Square mapping: `sq = rank * 12 + file` (a1=0, l1=11, a2=12, ..., l12=143)

use super::sq::SQ;

use std::fmt;
use std::ops::*;

/// Mask for the valid bits in word[2] (squares 128-143, 16 bits).
const WORD2_MASK: u64 = 0x0000_0000_0000_FFFF;

/// A 192-bit bitboard for the 12x12 board.
#[derive(Copy, Clone, Default, Hash, PartialEq, Eq)]
pub struct BitBoard(pub [u64; 3]);

impl BitBoard {
    /// Empty bitboard.
    pub const EMPTY: BitBoard = BitBoard([0, 0, 0]);

    /// All 144 valid squares set.
    pub const ALL: BitBoard = BitBoard([!0u64, !0u64, WORD2_MASK]);

    /// Creates a bitboard with a single square set.
    #[inline(always)]
    pub fn from_sq(sq: SQ) -> BitBoard {
        let mut bb = BitBoard::EMPTY;
        bb.set_bit(sq);
        bb
    }

    /// Sets a bit at the given square.
    #[inline(always)]
    pub fn set_bit(&mut self, sq: SQ) {
        let idx = sq.0 as usize;
        let word = idx / 64;
        let bit = idx % 64;
        self.0[word] |= 1u64 << bit;
    }

    /// Clears a bit at the given square.
    #[inline(always)]
    pub fn clear_bit(&mut self, sq: SQ) {
        let idx = sq.0 as usize;
        let word = idx / 64;
        let bit = idx % 64;
        self.0[word] &= !(1u64 << bit);
    }

    /// Tests if a bit is set at the given square.
    #[inline(always)]
    pub fn test_bit(self, sq: SQ) -> bool {
        let idx = sq.0 as usize;
        let word = idx / 64;
        let bit = idx % 64;
        (self.0[word] >> bit) & 1 != 0
    }

    /// Returns the number of set bits.
    #[inline(always)]
    pub fn count_bits(self) -> u32 {
        self.0[0].count_ones() + self.0[1].count_ones() + self.0[2].count_ones()
    }

    /// Returns true if the bitboard is empty.
    #[inline(always)]
    pub fn is_empty(self) -> bool {
        self.0[0] == 0 && self.0[1] == 0 && self.0[2] == 0
    }

    /// Returns true if the bitboard is not empty.
    #[inline(always)]
    pub fn is_not_empty(self) -> bool {
        !self.is_empty()
    }

    /// Returns true if more than one bit is set.
    #[inline(always)]
    pub fn more_than_one(self) -> bool {
        self.count_bits() > 1
    }

    /// Returns the SQ of the least significant bit.
    ///
    /// # Panics
    /// Panics if empty.
    #[inline(always)]
    pub fn bit_scan_forward(self) -> SQ {
        if self.0[0] != 0 {
            SQ(self.0[0].trailing_zeros() as u8)
        } else if self.0[1] != 0 {
            SQ(64 + self.0[1].trailing_zeros() as u8)
        } else {
            debug_assert!(self.0[2] != 0, "bit_scan_forward on empty bitboard");
            SQ(128 + self.0[2].trailing_zeros() as u8)
        }
    }

    /// Returns the SQ of the most significant bit.
    ///
    /// # Panics
    /// Panics if empty.
    #[inline]
    pub fn bit_scan_reverse(self) -> SQ {
        if self.0[2] != 0 {
            SQ(128 + (63 - self.0[2].leading_zeros() as u8))
        } else if self.0[1] != 0 {
            SQ(64 + (63 - self.0[1].leading_zeros() as u8))
        } else {
            debug_assert!(self.0[0] != 0, "bit_scan_reverse on empty bitboard");
            SQ(63 - self.0[0].leading_zeros() as u8)
        }
    }

    /// Converts a single-bit bitboard to its square.
    #[inline(always)]
    pub fn to_sq(self) -> SQ {
        self.bit_scan_forward()
    }

    /// Pops and returns the least significant bit as a SQ.
    #[inline(always)]
    pub fn pop_lsb(&mut self) -> SQ {
        let sq = self.bit_scan_forward();
        self.clear_bit(sq);
        sq
    }

    /// Pops the LSB if any.
    #[inline(always)]
    pub fn pop_some_lsb(&mut self) -> Option<SQ> {
        if self.is_empty() {
            None
        } else {
            Some(self.pop_lsb())
        }
    }

    /// Returns the front-most square for a player.
    #[inline]
    pub fn frontmost_sq(self, player: super::Player) -> SQ {
        match player {
            super::Player::White => self.bit_scan_reverse(),
            super::Player::Black => self.bit_scan_forward(),
        }
    }

    /// Returns the back-most square for a player.
    #[inline]
    pub fn backmost_sq(self, player: super::Player) -> SQ {
        match player {
            super::Player::White => self.bit_scan_forward(),
            super::Player::Black => self.bit_scan_reverse(),
        }
    }

    /// Shift the entire bitboard north by one rank (add 12).
    #[inline]
    pub fn shift_north(self) -> BitBoard {
        self << 12
    }

    /// Shift south by one rank (subtract 12).
    #[inline]
    pub fn shift_south(self) -> BitBoard {
        self >> 12
    }

    /// Mask to only valid squares (clear bits 144+).
    #[inline(always)]
    pub fn mask_valid(self) -> BitBoard {
        BitBoard([self.0[0], self.0[1], self.0[2] & WORD2_MASK])
    }
}

// === Bit operators ===

impl BitAnd for BitBoard {
    type Output = BitBoard;
    #[inline(always)]
    fn bitand(self, rhs: BitBoard) -> BitBoard {
        BitBoard([
            self.0[0] & rhs.0[0],
            self.0[1] & rhs.0[1],
            self.0[2] & rhs.0[2],
        ])
    }
}

impl BitAndAssign for BitBoard {
    #[inline(always)]
    fn bitand_assign(&mut self, rhs: BitBoard) {
        self.0[0] &= rhs.0[0];
        self.0[1] &= rhs.0[1];
        self.0[2] &= rhs.0[2];
    }
}

impl BitOr for BitBoard {
    type Output = BitBoard;
    #[inline(always)]
    fn bitor(self, rhs: BitBoard) -> BitBoard {
        BitBoard([
            self.0[0] | rhs.0[0],
            self.0[1] | rhs.0[1],
            self.0[2] | rhs.0[2],
        ])
    }
}

impl BitOrAssign for BitBoard {
    #[inline(always)]
    fn bitor_assign(&mut self, rhs: BitBoard) {
        self.0[0] |= rhs.0[0];
        self.0[1] |= rhs.0[1];
        self.0[2] |= rhs.0[2];
    }
}

impl BitXor for BitBoard {
    type Output = BitBoard;
    #[inline(always)]
    fn bitxor(self, rhs: BitBoard) -> BitBoard {
        BitBoard([
            self.0[0] ^ rhs.0[0],
            self.0[1] ^ rhs.0[1],
            self.0[2] ^ rhs.0[2],
        ])
    }
}

impl BitXorAssign for BitBoard {
    #[inline(always)]
    fn bitxor_assign(&mut self, rhs: BitBoard) {
        self.0[0] ^= rhs.0[0];
        self.0[1] ^= rhs.0[1];
        self.0[2] ^= rhs.0[2];
    }
}

impl Not for BitBoard {
    type Output = BitBoard;
    #[inline(always)]
    fn not(self) -> BitBoard {
        BitBoard([!self.0[0], !self.0[1], !self.0[2] & WORD2_MASK])
    }
}

impl Shl<u32> for BitBoard {
    type Output = BitBoard;
    #[inline]
    fn shl(self, shift: u32) -> BitBoard {
        if shift == 0 {
            return self;
        }
        if shift >= 192 {
            return BitBoard::EMPTY;
        }
        if shift >= 128 {
            let s = shift - 128;
            return BitBoard([0, 0, self.0[0] << s]).mask_valid();
        }
        if shift >= 64 {
            let s = shift - 64;
            if s == 0 {
                return BitBoard([0, self.0[0], self.0[1]]).mask_valid();
            }
            return BitBoard([
                0,
                self.0[0] << s,
                (self.0[1] << s) | (self.0[0] >> (64 - s)),
            ])
            .mask_valid();
        }
        // shift < 64
        BitBoard([
            self.0[0] << shift,
            (self.0[1] << shift) | (self.0[0] >> (64 - shift)),
            (self.0[2] << shift) | (self.0[1] >> (64 - shift)),
        ])
        .mask_valid()
    }
}

impl Shr<u32> for BitBoard {
    type Output = BitBoard;
    #[inline]
    fn shr(self, shift: u32) -> BitBoard {
        if shift == 0 {
            return self;
        }
        if shift >= 192 {
            return BitBoard::EMPTY;
        }
        if shift >= 128 {
            let s = shift - 128;
            return BitBoard([self.0[2] >> s, 0, 0]);
        }
        if shift >= 64 {
            let s = shift - 64;
            if s == 0 {
                return BitBoard([self.0[1], self.0[2], 0]);
            }
            return BitBoard([
                (self.0[1] >> s) | (self.0[2] << (64 - s)),
                self.0[2] >> s,
                0,
            ]);
        }
        // shift < 64
        BitBoard([
            (self.0[0] >> shift) | (self.0[1] << (64 - shift)),
            (self.0[1] >> shift) | (self.0[2] << (64 - shift)),
            self.0[2] >> shift,
        ])
    }
}

impl Sub<u64> for BitBoard {
    type Output = BitBoard;
    /// Subtract 1 (for pop_lsb pattern: bb & (bb - 1)).
    #[inline]
    fn sub(self, rhs: u64) -> BitBoard {
        debug_assert!(rhs == 1);
        // Subtract with borrow across words.
        let (w0, borrow0) = self.0[0].overflowing_sub(1);
        let (w1, borrow1) = self.0[1].overflowing_sub(borrow0 as u64);
        let w2 = self.0[2].wrapping_sub(borrow1 as u64);
        BitBoard([w0, w1, w2 & WORD2_MASK])
    }
}

impl Iterator for BitBoard {
    type Item = SQ;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        if self.is_empty() {
            None
        } else {
            Some(self.pop_lsb())
        }
    }
}

impl fmt::Debug for BitBoard {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "BitBoard({:#018x}, {:#018x}, {:#018x})",
            self.0[0], self.0[1], self.0[2]
        )
    }
}

impl fmt::Display for BitBoard {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for rank in (0..12u8).rev() {
            write!(f, "{:>2} ", rank + 1)?;
            for file in 0..12u8 {
                let sq = SQ(rank * 12 + file);
                if self.test_bit(sq) {
                    write!(f, "1 ")?;
                } else {
                    write!(f, ". ")?;
                }
            }
            writeln!(f)?;
        }
        write!(f, "   a b c d e f g h i j k l")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_set_clear_test_bit() {
        let mut bb = BitBoard::EMPTY;
        assert!(bb.is_empty());

        // Test across word boundaries
        for sq_val in [0u8, 1, 11, 63, 64, 65, 127, 128, 129, 143] {
            let sq = SQ(sq_val);
            bb.set_bit(sq);
            assert!(bb.test_bit(sq), "Failed to set bit at sq {}", sq_val);
        }
        assert_eq!(bb.count_bits(), 10);

        bb.clear_bit(SQ(64));
        assert!(!bb.test_bit(SQ(64)));
        assert_eq!(bb.count_bits(), 9);
    }

    #[test]
    fn test_bit_scan_forward() {
        let bb = BitBoard::from_sq(SQ(100));
        assert_eq!(bb.bit_scan_forward(), SQ(100));

        let bb2 = BitBoard::from_sq(SQ(0));
        assert_eq!(bb2.bit_scan_forward(), SQ(0));

        let bb3 = BitBoard::from_sq(SQ(143));
        assert_eq!(bb3.bit_scan_forward(), SQ(143));
    }

    #[test]
    fn test_pop_lsb() {
        let mut bb = BitBoard::EMPTY;
        bb.set_bit(SQ(10));
        bb.set_bit(SQ(70));
        bb.set_bit(SQ(130));

        let sq1 = bb.pop_lsb();
        assert_eq!(sq1, SQ(10));
        let sq2 = bb.pop_lsb();
        assert_eq!(sq2, SQ(70));
        let sq3 = bb.pop_lsb();
        assert_eq!(sq3, SQ(130));
        assert!(bb.is_empty());
    }

    #[test]
    fn test_shift_left_right() {
        // Shift by 12 (one rank north)
        let bb = BitBoard::from_sq(SQ(0));
        let shifted = bb << 12;
        assert!(shifted.test_bit(SQ(12)));
        assert!(!shifted.test_bit(SQ(0)));

        // Shift across word boundary
        let bb2 = BitBoard::from_sq(SQ(60));
        let shifted2 = bb2 << 12;
        assert!(shifted2.test_bit(SQ(72)));

        // Shift right
        let bb3 = BitBoard::from_sq(SQ(72));
        let shifted3 = bb3 >> 12;
        assert!(shifted3.test_bit(SQ(60)));
    }

    #[test]
    fn test_bitwise_ops() {
        let mut a = BitBoard::EMPTY;
        let mut b = BitBoard::EMPTY;
        a.set_bit(SQ(10));
        a.set_bit(SQ(20));
        b.set_bit(SQ(20));
        b.set_bit(SQ(30));

        let and = a & b;
        assert!(and.test_bit(SQ(20)));
        assert!(!and.test_bit(SQ(10)));
        assert!(!and.test_bit(SQ(30)));

        let or = a | b;
        assert!(or.test_bit(SQ(10)));
        assert!(or.test_bit(SQ(20)));
        assert!(or.test_bit(SQ(30)));

        let xor = a ^ b;
        assert!(xor.test_bit(SQ(10)));
        assert!(!xor.test_bit(SQ(20)));
        assert!(xor.test_bit(SQ(30)));
    }

    #[test]
    fn test_not() {
        let bb = BitBoard::EMPTY;
        let not_bb = !bb;
        assert_eq!(not_bb.count_bits(), 144);

        let all = BitBoard::ALL;
        let not_all = !all;
        assert!(not_all.is_empty());
    }

    #[test]
    fn test_iterator() {
        let mut bb = BitBoard::EMPTY;
        bb.set_bit(SQ(5));
        bb.set_bit(SQ(100));
        bb.set_bit(SQ(140));

        let sqs: Vec<SQ> = bb.collect();
        assert_eq!(sqs, vec![SQ(5), SQ(100), SQ(140)]);
    }

    #[test]
    fn test_word_boundary_63_64() {
        let mut bb = BitBoard::EMPTY;
        bb.set_bit(SQ(63));
        bb.set_bit(SQ(64));
        assert_eq!(bb.count_bits(), 2);
        assert!(bb.test_bit(SQ(63)));
        assert!(bb.test_bit(SQ(64)));

        let sq = bb.bit_scan_forward();
        assert_eq!(sq, SQ(63));
    }

    #[test]
    fn test_word_boundary_127_128() {
        let mut bb = BitBoard::EMPTY;
        bb.set_bit(SQ(127));
        bb.set_bit(SQ(128));
        assert_eq!(bb.count_bits(), 2);
        assert!(bb.test_bit(SQ(127)));
        assert!(bb.test_bit(SQ(128)));
    }
}
