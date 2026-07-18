//! Board state — immutable snapshot of game state for undo.

use super::castle_rights::CastleRights;
use crate::core::bitboard::BitBoard;
use crate::core::piece_identity::NO_PIECE_ID;
use crate::core::piece_move::HyperMove;
use crate::core::sq::{NO_SQ, SQ};
use crate::core::Piece;

/// Immutable state saved before each move for undo purposes.
#[derive(Clone)]
pub struct BoardState {
    /// Castling rights.
    pub castling: CastleRights,
    /// En passant target square (NO_SQ if none).
    pub ep_square: SQ,
    /// Half-move clock (for 112-move rule on 12×12 board; draw at 224 halfmoves).
    pub rule50: u16,
    /// Full move number.
    pub fullmove: u16,
    /// Zobrist hash.
    pub zobrist: u64,
    /// Piece captured on the last move (for undo).
    pub captured_piece: Piece,
    /// HFEN-I identity captured on the last move (for undo).
    pub captured_identity: char,
    /// Bitboard of pieces giving check to the side to move.
    pub checkers: BitBoard,
    /// The move that was played to reach this state.
    pub last_move: HyperMove,
    /// Previous state (for undo chain).
    pub prev: Option<Box<BoardState>>,
}

impl BoardState {
    pub fn new() -> Self {
        BoardState {
            castling: CastleRights::ALL,
            ep_square: NO_SQ,
            rule50: 0,
            fullmove: 1,
            zobrist: 0,
            captured_piece: Piece::None,
            captured_identity: NO_PIECE_ID,
            checkers: BitBoard::EMPTY,
            last_move: HyperMove::null(),
            prev: None,
        }
    }

    /// Creates a copy of this state to use as the "previous" state before a move.
    pub fn snapshot(&self) -> BoardState {
        BoardState {
            castling: self.castling,
            ep_square: self.ep_square,
            rule50: self.rule50,
            fullmove: self.fullmove,
            zobrist: self.zobrist,
            captured_piece: Piece::None,
            captured_identity: NO_PIECE_ID,
            checkers: BitBoard::EMPTY,
            last_move: HyperMove::null(),
            prev: None,
        }
    }
}

impl Default for BoardState {
    fn default() -> Self {
        BoardState::new()
    }
}
