// SPDX-License-Identifier: GPL-3.0-or-later
// HyperChess Core — hyperchess-rules
// File: crates/hyperchess-rules/src/board/movegen/king.rs
// Version: 1.0.0
// Copyright (c) 2026 HyperChess Developer Team

//! King move generation: ordinary king moves and castling.
//!
//! These produce *legal* moves directly (the king's own safety is checked here)
//! rather than going through the pseudo-legal → [`super::legality`] filter.

use crate::board::Board;
use crate::core::bitboard::BitBoard;
use crate::core::masks::*;
use crate::core::move_list::MoveList;
use crate::core::piece_move::HyperMove;
use crate::core::sq::SQ;
use crate::core::{Piece, Player};
use crate::helper::prelude::*;

use super::legality::squares_between_inclusive;

/// Generates king moves (non-castle). These go directly to the legal list.
pub(super) fn gen_king_moves(
    board: &Board,
    us: Player,
    our_occ: BitBoard,
    all_occ: BitBoard,
    list: &mut MoveList,
) {
    let king = board.king_sq(us);
    let attacks = king_attacks(king) & !our_occ;
    let them = !us;

    let mut targets = attacks;
    while let Some(dst) = targets.pop_some_lsb() {
        // Remove both the king (x-ray through its old square) and any captured piece
        // (so defenders behind the captured piece are not blocked) before checking safety.
        let mut temp_occ = all_occ;
        temp_occ.clear_bit(king);
        temp_occ.clear_bit(dst); // remove captured piece so its defenders are visible
        if board.attackers_to_player(dst, them, temp_occ).is_empty() {
            if board.piece_at(dst) != Piece::None {
                list.push(HyperMove::make_capture(king, dst));
            } else {
                list.push(HyperMove::make_quiet(king, dst));
            }
        }
    }
}

/// Generates castling moves. Goes directly to legal list.
pub(super) fn gen_castling(board: &Board, us: Player, all_occ: BitBoard, list: &mut MoveList) {
    let them = !us;
    let king = board.king_sq(us);

    // King-side castle
    if board.state.castling.can_king_side(us) {
        let rook_sq = SQ(CASTLING_ROOK_START[us as usize][0]);
        let king_dst = SQ(CASTLING_KING_DST[us as usize][0]);
        let rook_dst = SQ(CASTLING_ROOK_DST[us as usize][0]);

        if can_castle(board, us, them, king, rook_sq, king_dst, rook_dst, all_occ) {
            list.push(HyperMove::make_king_castle(king, king_dst));
        }
    }

    // Queen-side castle
    if board.state.castling.can_queen_side(us) {
        let rook_sq = SQ(CASTLING_ROOK_START[us as usize][1]);
        let king_dst = SQ(CASTLING_KING_DST[us as usize][1]);
        let rook_dst = SQ(CASTLING_ROOK_DST[us as usize][1]);

        if can_castle(board, us, them, king, rook_sq, king_dst, rook_dst, all_occ) {
            list.push(HyperMove::make_queen_castle(king, king_dst));
        }
    }
}

/// Checks if castling is possible given the current board state.
fn can_castle(
    board: &Board,
    _us: Player,
    them: Player,
    king_from: SQ,
    rook_from: SQ,
    king_to: SQ,
    rook_to: SQ,
    all_occ: BitBoard,
) -> bool {
    // The king must not be in check
    if board.in_check() {
        return false;
    }

    // All squares between king and its destination must be empty (except for king and rook positions)
    let king_path = squares_between_inclusive(king_from, king_to);
    let rook_path = squares_between_inclusive(rook_from, rook_to);

    // Combined path that must be clear (except for king and rook themselves)
    let mut must_be_clear = king_path | rook_path;
    must_be_clear.clear_bit(king_from);
    must_be_clear.clear_bit(rook_from);

    if (must_be_clear & all_occ).is_not_empty() {
        return false;
    }

    // King must not pass through or land on attacked squares
    let _king_min = king_from.0.min(king_to.0);
    let _king_max = king_from.0.max(king_to.0);
    let step: i16 = if king_to.0 > king_from.0 { 1 } else { -1 };

    let mut occ_without_king = all_occ;
    occ_without_king.clear_bit(king_from);
    occ_without_king.clear_bit(rook_from);

    let mut sq = king_from.0 as i16;
    while sq != king_to.0 as i16 + step {
        let check_sq = SQ(sq as u8);
        if board
            .attackers_to_player(check_sq, them, occ_without_king)
            .is_not_empty()
        {
            return false;
        }
        sq += step;
    }

    true
}
