// SPDX-License-Identifier: GPL-3.0-or-later
// HyperChess Core — hyperchess-search
// File: crates/hyperchess-search/src/search/ordering.rs
// Version: 1.0.0
// Copyright (c) 2026 HyperChess Developer Team

//! Move ordering shared by the alpha-beta family of searchers.
//!
//! Good ordering is what makes alpha-beta prune effectively: trying the best
//! move first maximises the number of beta cut-offs. Two strategies live here:
//!
//! * [`order_tactical`] — cheap, static ordering: hash move, then captures by
//!   MVV-LVA (Most Valuable Victim / Least Valuable Attacker), then promotions.
//! * [`order_by_eval`]  — expensive, near-perfect ordering: actually make each
//!   move and sort by the resulting static evaluation. Used by the "guided"
//!   searchers where the extra eval calls pay off at higher depths.

use hyperchess_rules::board::Board;
use hyperchess_rules::core::move_list::MoveList;
use hyperchess_rules::core::piece_move::HyperMove;
use hyperchess_rules::core::PieceType;
use hyperchess_rules::tools::eval::evaluate;

/// MVV-LVA score for a capture (0 for non-captures).
///
/// Captures are ranked by `victim_value * 100 - attacker_value`, biased above
/// quiet moves by a constant so every capture sorts ahead of every quiet move.
#[inline]
pub fn mvv_lva_score(board: &Board, m: HyperMove) -> i32 {
    if m.is_capture() {
        // `captured_piece_for_move` resolves en passant (destination is empty;
        // the victim pawn sits behind it) — reading the destination directly
        // would score EP as capturing nothing.
        let victim = board.captured_piece_for_move(m).type_of();
        let attacker = board.piece_at(m.get_src()).type_of();
        victim.value() as i32 * 100 - attacker.value() as i32 + 1000
    } else {
        0
    }
}

/// Static "tactical" ordering. Returns `(move, score)` pairs sorted best-first.
///
/// `tt_move` is the best move remembered from a shallower search of this node
/// (pass [`HyperMove::null`] when there is no transposition-table hint); it is
/// ordered first. Captures and promotions are then scored additively so a
/// capture-promotion outranks a plain capture.
pub fn order_tactical(
    board: &Board,
    moves: &MoveList,
    tt_move: HyperMove,
) -> Vec<(HyperMove, i32)> {
    let mut scored: Vec<(HyperMove, i32)> = moves
        .iter()
        .map(|&m| {
            let mut score = mvv_lva_score(board, m);
            if !tt_move.is_null() && m == tt_move {
                score += 10_000;
            }
            if m.is_promo() {
                score += 500;
            }
            (m, score)
        })
        .collect();
    scored.sort_unstable_by_key(|b| std::cmp::Reverse(b.1));
    scored
}

/// Near-perfect ordering: make each move and sort by the resulting evaluation
/// from the moving side's perspective. Requires `&mut Board` because it applies
/// and undoes each candidate move.
pub fn order_by_eval(board: &mut Board, moves: &MoveList) -> Vec<(HyperMove, i32)> {
    let mut scored: Vec<(HyperMove, i32)> = moves
        .iter()
        .map(|&m| {
            board.apply_move(m);
            // Child eval is from the child's perspective; negate for ours.
            let score = -evaluate(board);
            board.undo_move();
            (m, score)
        })
        .collect();
    scored.sort_unstable_by_key(|b| std::cmp::Reverse(b.1));
    scored
}

/// Move `tt_move` to the front of an already-ordered list, if present.
pub fn promote_tt_move(ordered: &mut Vec<(HyperMove, i32)>, tt_move: HyperMove) {
    if tt_move.is_null() {
        return;
    }
    if let Some(pos) = ordered.iter().position(|(m, _)| *m == tt_move) {
        let item = ordered.remove(pos);
        ordered.insert(0, item);
    }
}

/// "Raptor bonus" — HyperChess-specific ordering term for the jumping pieces.
///
/// A quiet Eagle or Hawk move that lands within Chebyshev distance 4 of the
/// enemy king moves the raptor into strike range: from there its jump attack
/// (up to 4 squares, over pieces) can deliver a check that **cannot be blocked
/// by interposition** — only a capture of the raptor or a king move answers it.
/// Trying these repositioning moves early finds king attacks (and therefore
/// beta cutoffs) sooner on the big 12×12 board, where a plain history table is
/// slow to discover them.
///
/// Kept small (+400) and inside the quiet band so it acts as a tie-break among
/// quiets, never outranking killers or countermoves.
#[inline]
pub fn raptor_bonus(board: &Board, m: HyperMove) -> i32 {
    let pt = board.piece_at(m.get_src()).type_of();
    if pt != PieceType::E && pt != PieceType::H {
        return 0;
    }
    let enemy_king = board.piece_bb(!board.turn(), PieceType::K);
    if !enemy_king.is_not_empty() {
        return 0; // no king on the board (analysis positions) — no target
    }
    let k = enemy_king.bit_scan_forward().0 as i32;
    let to = m.get_dest().0 as i32;
    let (kr, kf) = (k / 12, k % 12);
    let (tr, tf) = (to / 12, to % 12);
    let cheb = (kr - tr).abs().max((kf - tf).abs());
    if cheb <= 4 {
        400
    } else {
        0
    }
}

/// Score bands (highest-priority first):
///   1_000_000  TT/hash move
///     10_000+  captures (MVV-LVA offset)
///      9_000   promotions
///      8_000   killer 1
///      7_500   killer 2
///      7_250   countermove (refutation of the opponent's previous move)
///   ≤ 6_900    quiet (history score capped at ±6_500, plus raptor bonus ≤ 400)
///
/// History range: [-16_384, +16_384] after gravity. We cap at 6_500 (and the
/// raptor bonus at 400) so quiet moves never outrank the countermove or the
/// killers even when history is saturated.
pub fn order_full(
    board: &Board,
    moves: &MoveList,
    tt_move: HyperMove,
    history: &[i32], // flat 144×144, indexed [from*144 + to]
    killers: &[HyperMove; 2],
    counter: HyperMove, // countermove for the opponent's previous move (may be null)
) -> Vec<(HyperMove, i32)> {
    let mut scored: Vec<(HyperMove, i32)> = moves
        .iter()
        .map(|&m| {
            let score = if !tt_move.is_null() && m == tt_move {
                1_000_000
            } else if m.is_capture() {
                mvv_lva_score(board, m) + 10_000
            } else if m.is_promo() {
                9_000
            } else if killers[0] == m {
                8_000
            } else if killers[1] == m {
                7_500
            } else if !counter.is_null() && m == counter {
                7_250
            } else {
                let idx = m.get_src().0 as usize * 144 + m.get_dest().0 as usize;
                history.get(idx).copied().unwrap_or(0).clamp(-6_500, 6_500) + raptor_bonus(board, m)
            };
            (m, score)
        })
        .collect();
    scored.sort_unstable_by_key(|b| std::cmp::Reverse(b.1));
    scored
}

/// Apply a history bonus to the move that caused a beta cutoff.
///
/// Uses the "gravity" formula from Stockfish:
/// `h += bonus - |h| * bonus / MAX`
/// This keeps history in `[-MAX, +MAX]` without an explicit clamp.
pub fn history_bonus(history: &mut [i32], m: HyperMove, depth: i32) {
    const MAX: i32 = 16_384;
    let bonus = (depth * depth).min(2_048);
    let idx = m.get_src().0 as usize * 144 + m.get_dest().0 as usize;
    if let Some(h) = history.get_mut(idx) {
        *h += bonus - h.abs() * bonus / MAX;
    }
}

/// Apply a history penalty to a quiet move that was tried but failed to cut.
pub fn history_penalty(history: &mut [i32], m: HyperMove, depth: i32) {
    const MAX: i32 = 16_384;
    let penalty = (depth * depth).min(2_048);
    let idx = m.get_src().0 as usize * 144 + m.get_dest().0 as usize;
    if let Some(h) = history.get_mut(idx) {
        *h -= penalty - h.abs() * penalty / MAX;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn countermove_ranks_first_among_quiets() {
        // Start position: every legal move is quiet, history is empty, no TT
        // move, no killers. The countermove band (7_250) must therefore win.
        let board = Board::start_pos();
        let moves = board.generate_moves();
        let counter = *moves.iter().next().expect("start position has moves");
        let history = vec![0i32; 144 * 144];
        let killers = [HyperMove::null(); 2];
        let ordered = order_full(
            &board,
            &moves,
            HyperMove::null(),
            &history,
            &killers,
            counter,
        );
        assert_eq!(ordered[0].0, counter);
        assert_eq!(ordered[0].1, 7_250);
    }

    #[test]
    fn killers_outrank_countermove() {
        let board = Board::start_pos();
        let moves = board.generate_moves();
        let mut it = moves.iter();
        let killer = *it.next().unwrap();
        let counter = *it.next().unwrap();
        let history = vec![0i32; 144 * 144];
        let killers = [killer, HyperMove::null()];
        let ordered = order_full(
            &board,
            &moves,
            HyperMove::null(),
            &history,
            &killers,
            counter,
        );
        assert_eq!(
            ordered[0].0, killer,
            "killer 1 (8_000) beats countermove (7_250)"
        );
        assert_eq!(ordered[1].0, counter);
    }

    #[test]
    fn raptor_bonus_rewards_jumper_moves_into_king_zone() {
        // White Eagle on a8; Black king on a12. The quiet jump a8→a11 lands at
        // Chebyshev distance 1 from the enemy king (strike range) → +400.
        let board = Board::from_hfen("k11/12/12/12/E11/12/12/12/12/12/12/K11 w - - 0 1")
            .expect("raptor test HFEN should parse");
        let towards = board
            .generate_moves()
            .iter()
            .copied()
            .find(|m| m.stringify() == "a8a11")
            .expect("eagle jump a8a11 should be legal");
        assert_eq!(raptor_bonus(&board, towards), 400);

        // The retreating jump a8→a4 ends far from the king → no bonus.
        let away = board
            .generate_moves()
            .iter()
            .copied()
            .find(|m| m.stringify() == "a8a4")
            .expect("eagle jump a8a4 should be legal");
        assert_eq!(raptor_bonus(&board, away), 0);
    }

    #[test]
    fn raptor_bonus_ignores_non_jumpers() {
        // King moves are never raptor moves, wherever they land.
        let board = Board::from_hfen("k11/12/12/12/E11/12/12/12/12/12/12/K11 w - - 0 1")
            .expect("raptor test HFEN should parse");
        for m in board.generate_moves().iter().copied() {
            if board.piece_at(m.get_src()).type_of() == PieceType::K {
                assert_eq!(raptor_bonus(&board, m), 0);
            }
        }
    }

    #[test]
    fn saturated_history_stays_below_countermove_band() {
        // Even fully saturated history (+16_384) plus the raptor bonus must not
        // reach the countermove band at 7_250: cap is 6_500 + 400 = 6_900.
        let board = Board::start_pos();
        let moves = board.generate_moves();
        let history = vec![16_384i32; 144 * 144];
        let killers = [HyperMove::null(); 2];
        let ordered = order_full(
            &board,
            &moves,
            HyperMove::null(),
            &history,
            &killers,
            HyperMove::null(),
        );
        for (_, score) in &ordered {
            assert!(*score <= 6_900, "quiet band leaked: {score}");
        }
    }
}
