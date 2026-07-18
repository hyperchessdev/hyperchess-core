//! The `Player` (side) type and helpers.

use std::fmt;
use std::ops::Not;

use super::file_rank::Rank;
use super::masks::*;

/// Both players.
pub const ALL_PLAYERS: [Player; 2] = [Player::White, Player::Black];

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Player {
    White = 0,
    Black = 1,
}

impl Player {
    #[inline(always)]
    pub fn other_player(self) -> Player {
        !self
    }

    #[inline(always)]
    pub fn pawn_push(self) -> i16 {
        match self {
            Player::White => NORTH,
            Player::Black => SOUTH,
        }
    }

    /// Pawn start rank index.
    #[inline(always)]
    pub fn pawn_start_rank(self) -> u8 {
        match self {
            Player::White => 2, // rank 3 (0-indexed)
            Player::Black => 9, // rank 10 (0-indexed)
        }
    }

    /// Promotion rank index — the empty back rank the pawn steps onto.
    #[inline(always)]
    pub fn promotion_rank(self) -> u8 {
        match self {
            Player::White => 11, // rank 12 (0-indexed), the empty top row
            Player::Black => 0,  // rank 1 (0-indexed), the empty bottom row
        }
    }

    /// En passant capture rank index.
    #[inline(always)]
    pub fn ep_rank(self) -> u8 {
        match self {
            Player::White => 4, // rank 5 (0-indexed)
            Player::Black => 7, // rank 8 (0-indexed)
        }
    }

    /// Returns the relative rank from this player's perspective.
    #[inline]
    pub fn relative_rank(self, rank: Rank) -> Rank {
        let r = (rank as u8) ^ (self as u8 * 11);
        Rank::from_index(r)
    }

    /// Index for array access.
    #[inline(always)]
    pub fn index(self) -> usize {
        self as usize
    }
}

impl Not for Player {
    type Output = Player;
    fn not(self) -> Player {
        match self {
            Player::White => Player::Black,
            Player::Black => Player::White,
        }
    }
}

impl fmt::Display for Player {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Player::White => write!(f, "White"),
            Player::Black => write!(f, "Black"),
        }
    }
}
