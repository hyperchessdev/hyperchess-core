//! Pawn move generation: pushes, double pushes, captures, en passant, promotion.

use crate::board::Board;
use crate::core::bitboard::BitBoard;
use crate::core::move_list::MoveList;
use crate::core::piece_move::HyperMove;
use crate::core::sq::SQ;
use crate::core::{PieceType, Player};
use crate::helper::prelude::*;

/// Generates pawn moves (push, double push, capture, en passant, promotion).
pub(super) fn gen_pawn_moves(
    board: &Board,
    us: Player,
    _our_occ: BitBoard,
    their_occ: BitBoard,
    all_occ: BitBoard,
    list: &mut MoveList,
) {
    let push_dir = us.pawn_push();
    let start_rank = us.pawn_start_rank();
    let promo_rank = us.promotion_rank();

    let mut pawns = board.piece_bb(us, PieceType::P);
    while let Some(src) = pawns.pop_some_lsb() {
        let rank = src.rank_idx();

        // Single push
        let push_sq_val = src.0 as i16 + push_dir;
        if (0..144).contains(&push_sq_val) {
            let push_sq = SQ(push_sq_val as u8);
            if !all_occ.test_bit(push_sq) {
                if push_sq.rank_idx() == promo_rank {
                    // Promotion
                    for &promo_pt in PieceType::PROMO_TYPES.iter() {
                        list.push(HyperMove::make_promotion(src, push_sq, promo_pt, false));
                    }
                } else {
                    list.push(HyperMove::make_quiet(src, push_sq));

                    // Double push from start rank
                    if rank == start_rank {
                        let double_sq_val = push_sq_val + push_dir;
                        if (0..144).contains(&double_sq_val) {
                            let double_sq = SQ(double_sq_val as u8);
                            if !all_occ.test_bit(double_sq) {
                                list.push(HyperMove::make_pawn_push(src, double_sq));
                            }
                        }
                    }
                }
            }
        }

        // Captures
        let attacks = pawn_attacks_sq(us, src);
        let mut captures = attacks & their_occ;
        while let Some(dst) = captures.pop_some_lsb() {
            if dst.rank_idx() == promo_rank {
                for &promo_pt in PieceType::PROMO_TYPES.iter() {
                    list.push(HyperMove::make_promotion(src, dst, promo_pt, true));
                }
            } else {
                list.push(HyperMove::make_capture(src, dst));
            }
        }

        // En passant
        let ep_sq = board.state.ep_square;
        if ep_sq.is_okay() && attacks.test_bit(ep_sq) {
            list.push(HyperMove::make_ep_capture(src, ep_sq));
        }
    }
}
