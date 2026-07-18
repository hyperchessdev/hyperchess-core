//! Move generation for the non-pawn, non-king pieces:
//! Knight, Bishop, Rook, Queen, Eagle, Hawk.

use crate::board::Board;
use crate::core::bitboard::BitBoard;
use crate::core::move_list::MoveList;
use crate::core::piece_move::HyperMove;
use crate::core::{Piece, PieceType, Player};
use crate::helper::prelude::*;

/// Generates pseudo-legal moves for a non-pawn, non-king piece type.
pub(super) fn gen_piece_moves(
    board: &Board,
    us: Player,
    pt: PieceType,
    our_occ: BitBoard,
    all_occ: BitBoard,
    list: &mut MoveList,
) {
    let mut pieces = board.piece_bb(us, pt);
    while let Some(src) = pieces.pop_some_lsb() {
        let attacks = piece_attacks(pt, src, all_occ) & !our_occ;
        let mut targets = attacks;
        while let Some(dst) = targets.pop_some_lsb() {
            if board.piece_at(dst) != Piece::None {
                list.push(HyperMove::make_capture(src, dst));
            } else {
                list.push(HyperMove::make_quiet(src, dst));
            }
        }
    }
}
