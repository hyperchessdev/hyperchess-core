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

/// Score bands (highest-priority first):
///   1_000_000  TT/hash move
///     10_000+  captures (MVV-LVA offset)
///      9_000   promotions
///      8_000   killer 1
///      7_500   killer 2
///   ≤ 7_000    quiet (history score, capped)
///
/// History range: [-16_384, +16_384] after gravity.  We cap at 7_000 so quiet
/// moves never outrank killers even when history is saturated.
pub fn order_full(
    board: &Board,
    moves: &MoveList,
    tt_move: HyperMove,
    history: &[i32], // flat 144×144, indexed [from*144 + to]
    killers: &[HyperMove; 2],
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
            } else {
                let idx = m.get_src().0 as usize * 144 + m.get_dest().0 as usize;
                history.get(idx).copied().unwrap_or(0).clamp(-7_000, 7_000)
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
