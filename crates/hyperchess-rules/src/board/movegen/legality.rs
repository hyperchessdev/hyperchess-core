//! Legality filtering for pseudo-legal moves and ray helpers.

use crate::board::Board;
use crate::core::bitboard::BitBoard;
use crate::core::piece_move::HyperMove;
use crate::core::sq::SQ;
use crate::core::Player;
use crate::helper::prelude::*;

/// Returns a bitboard of squares from `a` to `b` inclusive
/// (only meaningful for squares on the same rank/file/diagonal).
pub(super) fn squares_between_inclusive(a: SQ, b: SQ) -> BitBoard {
    let mut bb = between(a, b);
    bb.set_bit(a);
    bb.set_bit(b);
    bb
}

/// Checks if a pseudo-legal move is legal (doesn't leave own king in check).
pub(super) fn is_legal(board: &Board, m: HyperMove, us: Player, king: SQ) -> bool {
    let src = m.get_src();
    let dst = m.get_dest();
    let them = !us;

    // En passant is tricky: might expose king to check along the rank
    if m.is_en_passant() {
        let captured_sq = SQ((dst.0 as i16 - us.pawn_push()) as u8);
        let mut occ = board.occupied();
        occ.clear_bit(src);
        occ.clear_bit(captured_sq);
        occ.set_bit(dst);

        return board.attackers_to_player(king, them, occ).is_empty();
    }

    let mut occ = board.occupied();
    occ.clear_bit(src);
    occ.set_bit(dst);

    let attackers = board.attackers_to_player(king, them, occ);

    // For captures: the enemy piece at dst is removed from the board.
    // attackers_to_player uses the board's piece bitboards (unchanged), so the
    // captured piece at dst may still appear as an attacker. Mask it out.
    if m.is_capture() {
        (attackers & !BitBoard::from_sq(dst)).is_empty()
    } else {
        attackers.is_empty()
    }
}

/// Returns a bitboard of squares attacked by a player (for testing).
#[cfg(test)]
pub fn attacked_squares(board: &Board, player: Player) -> BitBoard {
    use crate::core::PieceType;

    let mut attacked = BitBoard::EMPTY;
    let occ = board.occupied();

    // Pawns
    let mut pawns = board.piece_bb(player, PieceType::P);
    while let Some(sq) = pawns.pop_some_lsb() {
        attacked |= pawn_attacks_sq(player, sq);
    }

    // Pieces
    for &pt in &[
        PieceType::N,
        PieceType::B,
        PieceType::R,
        PieceType::Q,
        PieceType::K,
        PieceType::E,
        PieceType::H,
    ] {
        let mut pieces = board.piece_bb(player, pt);
        while let Some(sq) = pieces.pop_some_lsb() {
            attacked |= piece_attacks(pt, sq, occ);
        }
    }

    attacked
}
