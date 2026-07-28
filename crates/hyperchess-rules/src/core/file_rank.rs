// SPDX-License-Identifier: GPL-3.0-or-later
// HyperChess Core — hyperchess-rules
// File: crates/hyperchess-rules/src/core/file_rank.rs
// Version: 1.0.0
// Copyright (c) 2026 HyperChess Developer Team

//! The `File` and `Rank` coordinate enums for the 12×12 board.

use super::masks::*;

/// All files.
pub static ALL_FILES: [File; FILE_CNT] = [
    File::A,
    File::B,
    File::C,
    File::D,
    File::E,
    File::F,
    File::G,
    File::H,
    File::I,
    File::J,
    File::K,
    File::L,
];

/// All ranks.
pub static ALL_RANKS: [Rank; RANK_CNT] = [
    Rank::R1,
    Rank::R2,
    Rank::R3,
    Rank::R4,
    Rank::R5,
    Rank::R6,
    Rank::R7,
    Rank::R8,
    Rank::R9,
    Rank::R10,
    Rank::R11,
    Rank::R12,
];

#[repr(u8)]
#[derive(Copy, Clone, PartialEq, Eq, Debug, Ord, PartialOrd, Hash)]
/// A board column, `A`..=`L`. HyperChess is 12 wide, so files run two past
/// standard chess's `H`.
///
/// Discriminants are the 0-based column index, which is what lets file/rank
/// pairs convert to a square as `rank * 12 + file` without a lookup.
pub enum File {
    A = 0,
    B = 1,
    C = 2,
    D = 3,
    E = 4,
    F = 5,
    G = 6,
    H = 7,
    I = 8,
    J = 9,
    K = 10,
    L = 11,
}

impl File {
    /// Column index (0-11) to `File`.
    ///
    /// Debug-asserts the range and would panic on the array index in release —
    /// callers derive `idx` from a square, so an out-of-range value means the
    /// square itself was already corrupt.
    #[inline]
    pub fn from_index(idx: u8) -> File {
        debug_assert!(idx < 12);
        ALL_FILES[idx as usize]
    }

    /// Absolute column separation, computed without signed arithmetic since
    /// the discriminants are `u8`.
    pub fn distance(self, other: File) -> u8 {
        if self > other {
            self as u8 - other as u8
        } else {
            other as u8 - self as u8
        }
    }

    /// Display letter for this file (`'a'`..=`'l'`), as used by HFEN and UCI.
    pub fn char(self) -> char {
        FILE_DISPLAYS[self as usize]
    }
}

#[repr(u8)]
#[derive(Copy, Clone, PartialEq, Eq, Debug, Ord, PartialOrd, Hash)]
/// A board row, `R1`..=`R12`, with discriminants as the 0-based row index.
///
/// `R1` is White's back rank and `R12` is Black's; the enum is ordered from
/// White's side, so `Ord` on `Rank` means "further from White".
pub enum Rank {
    R1 = 0,
    R2 = 1,
    R3 = 2,
    R4 = 3,
    R5 = 4,
    R6 = 5,
    R7 = 6,
    R8 = 7,
    R9 = 8,
    R10 = 9,
    R11 = 10,
    R12 = 11,
}

impl Rank {
    /// Row index (0-11) to `Rank`. Same range contract as [`File::from_index`].
    #[inline]
    pub fn from_index(idx: u8) -> Rank {
        debug_assert!(idx < 12, "Rank::from_index called with {}", idx);
        ALL_RANKS[idx as usize]
    }

    /// Absolute row separation.
    pub fn distance(self, other: Rank) -> u8 {
        if self > other {
            self as u8 - other as u8
        } else {
            other as u8 - self as u8
        }
    }
}
