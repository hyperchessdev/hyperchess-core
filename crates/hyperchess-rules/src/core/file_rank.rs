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
    #[inline]
    pub fn from_index(idx: u8) -> File {
        debug_assert!(idx < 12);
        ALL_FILES[idx as usize]
    }

    pub fn distance(self, other: File) -> u8 {
        if self > other {
            self as u8 - other as u8
        } else {
            other as u8 - self as u8
        }
    }

    pub fn char(self) -> char {
        FILE_DISPLAYS[self as usize]
    }
}

#[repr(u8)]
#[derive(Copy, Clone, PartialEq, Eq, Debug, Ord, PartialOrd, Hash)]
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
    #[inline]
    pub fn from_index(idx: u8) -> Rank {
        debug_assert!(idx < 12, "Rank::from_index called with {}", idx);
        ALL_RANKS[idx as usize]
    }

    pub fn distance(self, other: Rank) -> u8 {
        if self > other {
            self as u8 - other as u8
        } else {
            other as u8 - self as u8
        }
    }
}
