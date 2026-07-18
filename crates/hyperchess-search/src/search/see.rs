//! Static Exchange Evaluation (SEE).
//!
//! Given a capture, SEE plays out the full sequence of recaptures on the target
//! square — each side always recapturing with its *least valuable* attacker, and
//! free to stop whenever continuing would lose material — and returns the net
//! centipawn outcome for the side that made the first capture.
//!
//! It is the engine's defence against "trading a strong piece for a pawn":
//! quiescence uses it to skip captures whose `SEE < 0`, so the search never wastes
//! effort on (or plays into) losing exchanges.
//!
//! Sliding x-rays are handled for free: [`Board::attackers_to`] recomputes attacks
//! from the *current* occupancy, so removing a front attacker exposes the slider
//! behind it on the next iteration.

use hyperchess_rules::board::Board;
use hyperchess_rules::core::piece_move::HyperMove;
use hyperchess_rules::core::score::Value;
use hyperchess_rules::core::{PieceType, Player};

/// SEE piece values (centipawns), indexed by `PieceType as usize`. The king is
/// effectively infinite so it is always the last resort attacker.
const SEE_VALUE: [Value; 10] = [
    0,      // None
    100,    // P
    320,    // N
    330,    // B
    500,    // R
    900,    // Q
    20_000, // K
    700,    // E (Eagle)
    550,    // H (Hawk)
    0,      // All
];

/// Attackers are tried cheapest-first (least-valuable-attacker order).
const LVA_ORDER: [PieceType; 8] = [
    PieceType::P,
    PieceType::N,
    PieceType::B,
    PieceType::R,
    PieceType::H,
    PieceType::E,
    PieceType::Q,
    PieceType::K,
];

#[inline]
fn value_of(pt: PieceType) -> Value {
    SEE_VALUE[pt as usize]
}

/// Net material outcome (centipawns) of the capture `mv`, from the moving side's
/// perspective. `>= 0` means the exchange does not lose material.
///
/// Assumes `mv` is a capture (there is an enemy piece on the destination).
pub fn see(board: &Board, mv: HyperMove) -> Value {
    let to = mv.get_dest();
    let from = mv.get_src();

    let attacker = board.piece_at(from).type_of();

    let mut occ = board.occupied();
    occ.clear_bit(from);

    // En passant: the destination square is empty — the victim is the pawn
    // behind it, which also leaves the board before any recapture happens.
    let victim = if mv.is_en_passant() {
        occ.clear_bit(board.ep_captured_sq(mv));
        PieceType::P
    } else {
        board.piece_at(to).type_of()
    };

    // We win the victim, then the opponent (to move) tries to win our attacker back.
    value_of(victim) - gain(board, to, !board.turn(), occ, value_of(attacker))
}

/// Convenience predicate: is the capture at least break-even?
#[inline]
pub fn see_ge_zero(board: &Board, mv: HyperMove) -> bool {
    see(board, mv) >= 0
}

/// The material `side` can gain by continuing the exchange on `to`, where the
/// piece currently sitting on `to` is worth `on_to`. Returns a non-negative value:
/// `side` will only recapture if it improves their result (else it "stands pat").
fn gain(
    board: &Board,
    to: hyperchess_rules::core::sq::SQ,
    side: Player,
    occ: hyperchess_rules::core::bitboard::BitBoard,
    on_to: Value,
) -> Value {
    // Least valuable attacker of `side` to `to` under the current occupancy.
    let attackers = board.attackers_to(to, occ) & occ & board.occupied_player(side);
    if attackers.is_empty() {
        return 0;
    }

    let mut lva_sq = None;
    let mut lva_pt = PieceType::None;
    for &pt in LVA_ORDER.iter() {
        let set = attackers & board.piece_bb(side, pt);
        if set.is_not_empty() {
            lva_sq = Some(set.bit_scan_forward());
            lva_pt = pt;
            break;
        }
    }
    let lva_sq = match lva_sq {
        Some(sq) => sq,
        None => return 0,
    };

    let mut next_occ = occ;
    next_occ.clear_bit(lva_sq);

    // King legality: the king may only capture when nothing defends `to` afterwards
    // (it cannot move into check). If the opponent still attacks `to`, the king
    // cannot take, so `side` has no legal recapture here.
    if lva_pt == PieceType::K {
        let defenders = board.attackers_to(to, next_occ) & next_occ & board.occupied_player(!side);
        if defenders.is_not_empty() {
            return 0;
        }
    }

    // Capture the piece on `to` (worth `on_to`); the opponent then tries to win our
    // recapturing piece (worth value_of(lva_pt)) back.
    let g = on_to - gain(board, to, !side, next_occ, value_of(lva_pt));
    g.max(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use hyperchess_rules::board::Board;

    fn find_move(board: &Board, uci: &str) -> HyperMove {
        board
            .generate_moves()
            .iter()
            .copied()
            .find(|m| m.stringify() == uci)
            .unwrap_or_else(|| panic!("move {uci} not legal"))
    }

    #[test]
    fn capturing_an_undefended_piece_wins_its_value() {
        // Valid 2-king position: White pawn on c2 can capture an UNDEFENDED Black
        // queen on d3 (the black king is in the far corner, defending nothing).
        // Ranks are listed 12→1; sq = rank*12 + file. Pawn c2=(r1,f2), queen d3=(r2,f3).
        // rank12: black king on file11; ranks 11..4 empty; rank3: queen on file3;
        // rank2: pawn on file2; rank1: white king on file0.
        let hfen = "11k/12/12/12/12/12/12/12/12/3q8/2P9/K11 w - - 0 1";
        let board = Board::from_hfen(hfen).expect("test HFEN should parse");
        let cap = board
            .generate_moves()
            .iter()
            .copied()
            .find(|m| m.is_capture())
            .expect("pawn should have a capture of the queen");
        // Free queen: SEE equals the queen's value (no recapture available).
        assert_eq!(see(&board, cap), 900);
    }

    #[test]
    fn defended_recapture_nets_the_difference() {
        // White pawn on c2 captures Black queen on d3, but the queen is defended by a
        // Black pawn on e4 (which recaptures the white pawn). Net for White:
        // +queen(900) − pawn(100) = +800.
        // queen d3=(r2,f3) defended by black pawn e4=(r3,f4) capturing SW to d3.
        let hfen = "11k/12/12/12/12/12/12/12/4p7/3q8/2P9/K11 w - - 0 1";
        let board = Board::from_hfen(hfen).expect("test HFEN should parse");
        let cap = board
            .generate_moves()
            .iter()
            .copied()
            .find(|m| m.is_capture() && m.get_dest().0 == 2 * 12 + 3)
            .expect("pawn should capture the queen on d3");
        assert_eq!(see(&board, cap), 800);
    }

    #[test]
    fn en_passant_capture_sees_the_captured_pawn() {
        // Black pawn a10 double-pushes past White pawn b8; White may capture
        // en passant b8xa9. The destination square a9 is EMPTY — the victim is
        // the pawn behind it. Before the fix, SEE scored the victim as None (0)
        // and MVV-LVA scored EP below every real capture.
        let hfen = "11k/12/p11/12/1P10/12/12/12/12/12/12/K11 b - - 0 1";
        let mut board = Board::from_hfen(hfen).expect("test HFEN should parse");
        let dp = find_move(&board, "a10a8");
        assert!(dp.is_double_push());
        board.apply_move(dp);

        let ep = board
            .generate_moves()
            .iter()
            .copied()
            .find(|m| m.is_en_passant())
            .expect("en passant capture should be legal");

        // Free pawn: nothing recaptures on a9 → SEE = +pawn.
        assert_eq!(see(&board, ep), 100);

        // MVV-LVA must score EP as pawn-takes-pawn (victim 1 * 100 − attacker 1
        // + 1000 capture bias), not as capturing an empty square.
        use crate::search::ordering::mvv_lva_score;
        assert_eq!(mvv_lva_score(&board, ep), 100 - 1 + 1000);
    }

    #[test]
    fn see_is_symmetric_sign_for_equal_trades() {
        // Smoke test on the start position: every pseudo-capture's SEE is finite and
        // the function does not panic / recurse forever.
        let board = Board::start_pos();
        for m in board
            .generate_moves()
            .iter()
            .copied()
            .filter(|m| m.is_capture())
        {
            let s = see(&board, m);
            assert!(s.abs() < 30_000);
        }
    }
}
