// SPDX-License-Identifier: GPL-3.0-or-later
// HyperChess Core — hyperchess-rules
// File: crates/hyperchess-rules/src/helper/prelude.rs
// Version: 1.0.0
// Copyright (c) 2026 HyperChess Developer Team

//! Initialization and public accessor functions for the helper module.

use crate::core::bitboard::BitBoard;
use crate::core::sq::SQ;
use crate::core::{PieceType, Player};

use super::boards;
use super::jumping;
use super::psqt;
use super::sliding;

use std::sync::Once;

static INIT: Once = Once::new();

/// Initialize all static lookup tables. Safe to call multiple times.
pub fn init_statics() {
    INIT.call_once(|| {
        boards::init_boards();
        psqt::init_psqt();
        // Zobrist keys are lazily initialized via lazy_static
    });
}

// Re-export accessors

/// Knight attacks from a square.
#[inline(always)]
pub fn knight_attacks(sq: SQ) -> BitBoard {
    boards::knight_attacks(sq)
}

/// King attacks from a square.
#[inline(always)]
pub fn king_attacks(sq: SQ) -> BitBoard {
    boards::king_attacks(sq)
}

/// Pawn attacks for a player from a square.
#[inline(always)]
pub fn pawn_attacks_sq(player: Player, sq: SQ) -> BitBoard {
    boards::pawn_attacks(player, sq)
}

/// Rook attacks (sliding).
#[inline(always)]
pub fn rook_attacks(sq: SQ, occupied: BitBoard) -> BitBoard {
    sliding::rook_attacks(sq, occupied)
}

/// Bishop attacks (sliding).
#[inline(always)]
pub fn bishop_attacks(sq: SQ, occupied: BitBoard) -> BitBoard {
    sliding::bishop_attacks(sq, occupied)
}

/// Queen attacks (sliding).
#[inline(always)]
pub fn queen_attacks(sq: SQ, occupied: BitBoard) -> BitBoard {
    sliding::queen_attacks(sq, occupied)
}

/// Eagle attacks (jumping, orthogonal).
#[inline(always)]
pub fn eagle_attacks(sq: SQ, occupied: BitBoard) -> BitBoard {
    jumping::eagle_attacks(sq, occupied)
}

/// Hawk attacks (jumping, diagonal).
#[inline(always)]
pub fn hawk_attacks(sq: SQ, occupied: BitBoard) -> BitBoard {
    jumping::hawk_attacks(sq, occupied)
}

/// Returns attacks for a given piece type from a given square.
#[inline]
pub fn piece_attacks(piece_type: PieceType, sq: SQ, occupied: BitBoard) -> BitBoard {
    match piece_type {
        PieceType::N => knight_attacks(sq),
        PieceType::B => bishop_attacks(sq, occupied),
        PieceType::R => rook_attacks(sq, occupied),
        PieceType::Q => queen_attacks(sq, occupied),
        PieceType::K => king_attacks(sq),
        PieceType::E => eagle_attacks(sq, occupied),
        PieceType::H => hawk_attacks(sq, occupied),
        _ => BitBoard::EMPTY,
    }
}

/// Distance between two squares.
#[inline(always)]
pub fn distance(sq1: SQ, sq2: SQ) -> u8 {
    boards::distance(sq1, sq2)
}

/// Line through two squares.
#[inline(always)]
pub fn line(sq1: SQ, sq2: SQ) -> BitBoard {
    boards::line(sq1, sq2)
}

/// Squares between two aligned squares (exclusive).
#[inline(always)]
pub fn between(sq1: SQ, sq2: SQ) -> BitBoard {
    boards::between(sq1, sq2)
}

/// Whether three squares are aligned (on the same line).
#[inline(always)]
pub fn aligned(sq1: SQ, sq2: SQ, sq3: SQ) -> bool {
    boards::aligned(sq1, sq2, sq3)
}
