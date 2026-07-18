//! Castling rights for HyperChess (12x12 board).

use crate::core::masks::*;
use crate::core::sq::SQ;
use crate::core::Player;

/// Castling rights stored as a u8 bitmask.
/// Bit 3: White king-side
/// Bit 2: White queen-side
/// Bit 1: Black king-side
/// Bit 0: Black queen-side
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct CastleRights(pub u8);

impl CastleRights {
    pub const NONE: CastleRights = CastleRights(0);
    pub const ALL: CastleRights = CastleRights(0b1111);

    pub const WHITE_KING: CastleRights = CastleRights(C_WHITE_K_MASK);
    pub const WHITE_QUEEN: CastleRights = CastleRights(C_WHITE_Q_MASK);
    pub const BLACK_KING: CastleRights = CastleRights(C_BLACK_K_MASK);
    pub const BLACK_QUEEN: CastleRights = CastleRights(C_BLACK_Q_MASK);
    pub const WHITE_BOTH: CastleRights = CastleRights(C_WHITE_K_MASK | C_WHITE_Q_MASK);
    pub const BLACK_BOTH: CastleRights = CastleRights(C_BLACK_K_MASK | C_BLACK_Q_MASK);

    /// Can the given player castle king-side?
    #[inline(always)]
    pub fn can_king_side(self, player: Player) -> bool {
        let mask = match player {
            Player::White => C_WHITE_K_MASK,
            Player::Black => C_BLACK_K_MASK,
        };
        self.0 & mask != 0
    }

    /// Can the given player castle queen-side?
    #[inline(always)]
    pub fn can_queen_side(self, player: Player) -> bool {
        let mask = match player {
            Player::White => C_WHITE_Q_MASK,
            Player::Black => C_BLACK_Q_MASK,
        };
        self.0 & mask != 0
    }

    /// Has any castling right.
    #[inline(always)]
    pub fn has_any(self) -> bool {
        self.0 != 0
    }

    /// Remove a specific right.
    #[inline(always)]
    pub fn remove(&mut self, right: CastleRights) {
        self.0 &= !right.0;
    }

    /// Remove all rights for a player.
    #[inline(always)]
    pub fn remove_player(&mut self, player: Player) {
        match player {
            Player::White => self.0 &= !(C_WHITE_K_MASK | C_WHITE_Q_MASK),
            Player::Black => self.0 &= !(C_BLACK_K_MASK | C_BLACK_Q_MASK),
        }
    }

    /// Update castling rights when a piece moves from/to a square.
    /// Should be called for every move to handle king/rook moves.
    pub fn update_for_move(&mut self, from: SQ, to: SQ) {
        // If king moves, remove both castling rights for that player
        if from.0 == WHITE_KING_START || to.0 == WHITE_KING_START {
            self.0 &= !(C_WHITE_K_MASK | C_WHITE_Q_MASK);
        }
        if from.0 == BLACK_KING_START || to.0 == BLACK_KING_START {
            self.0 &= !(C_BLACK_K_MASK | C_BLACK_Q_MASK);
        }

        // If rook moves or is captured, remove that side's castling right
        if from.0 == ROOK_WHITE_KSIDE_START || to.0 == ROOK_WHITE_KSIDE_START {
            self.0 &= !C_WHITE_K_MASK;
        }
        if from.0 == ROOK_WHITE_QSIDE_START || to.0 == ROOK_WHITE_QSIDE_START {
            self.0 &= !C_WHITE_Q_MASK;
        }
        if from.0 == ROOK_BLACK_KSIDE_START || to.0 == ROOK_BLACK_KSIDE_START {
            self.0 &= !C_BLACK_K_MASK;
        }
        if from.0 == ROOK_BLACK_QSIDE_START || to.0 == ROOK_BLACK_QSIDE_START {
            self.0 &= !C_BLACK_Q_MASK;
        }
    }

    /// Parse from HFEN string (e.g., "KQkq", "Kq", "-").
    pub fn from_hfen(s: &str) -> CastleRights {
        if s == "-" {
            return CastleRights::NONE;
        }
        let mut rights = 0u8;
        for c in s.chars() {
            match c {
                'K' => rights |= C_WHITE_K_MASK,
                'Q' => rights |= C_WHITE_Q_MASK,
                'k' => rights |= C_BLACK_K_MASK,
                'q' => rights |= C_BLACK_Q_MASK,
                _ => {}
            }
        }
        CastleRights(rights)
    }

    /// Convert to HFEN string.
    pub fn to_hfen(self) -> String {
        if self.0 == 0 {
            return String::from("-");
        }
        let mut s = String::new();
        if self.0 & C_WHITE_K_MASK != 0 {
            s.push('K');
        }
        if self.0 & C_WHITE_Q_MASK != 0 {
            s.push('Q');
        }
        if self.0 & C_BLACK_K_MASK != 0 {
            s.push('k');
        }
        if self.0 & C_BLACK_Q_MASK != 0 {
            s.push('q');
        }
        s
    }
}

impl Default for CastleRights {
    fn default() -> Self {
        CastleRights::ALL
    }
}
