//! Quiescence search — extends the search at leaf nodes through captures only,
//! avoiding the horizon effect where a search stops in the middle of a trade.
//!
//! Two correctness rules beyond plain capture search:
//!
//! * **In check there is no stand-pat.** The side to move is forced to respond,
//!   so every evasion is searched; if none exists the node is checkmate and is
//!   scored as a mate (by distance from the root), not by the static eval.
//! * **The caller's budget applies here too.** `tick` is invoked once per node
//!   so quiescence nodes count toward node limits and the wall-clock deadline —
//!   a capture cascade can no longer overshoot the budget unnoticed.

use crate::search::see::see;
use hyperchess_rules::board::Board;
use hyperchess_rules::core::score::*;
use hyperchess_rules::tools::eval::evaluate;

/// Hard ceiling on quiescence recursion (in plies from the root), guarding
/// against pathological capture/check chains. Well above anything reachable in
/// practice, and low enough that mate scores stay inside
/// `VALUE_MATE_IN_MAX_PLY` (`VALUE_MATE - 256`).
const QS_MAX_PLY: usize = 200;

/// Capture-only negamax quiescence search (full-width evasions when in check).
///
/// Returns a fail-hard bound in `[alpha, beta]`. `ply` is the distance from the
/// search root (used for mate scores and the recursion cap). `delta_prune`
/// additionally skips captures that cannot lift stand-pat near alpha even if
/// they win the victim outright (the Pro profile's delta pruning). `tick` is
/// called once per node and must return `true` when the search should abort; on
/// abort the current `alpha` is returned and the caller discards the value.
pub fn quiesce(
    board: &mut Board,
    mut alpha: Value,
    beta: Value,
    ply: usize,
    delta_prune: bool,
    tick: &mut impl FnMut() -> bool,
) -> Value {
    if tick() {
        return alpha;
    }

    // Forced response: no stand-pat while in check. Search every evasion; if
    // none exists this is checkmate, preferred sooner (smaller ply) at the root.
    if board.in_check() {
        let moves = board.generate_moves();
        if moves.is_empty() {
            return -VALUE_MATE + ply as Value;
        }
        if ply >= QS_MAX_PLY {
            return evaluate(board);
        }
        for m in moves.iter() {
            board.apply_move(*m);
            let score = -quiesce(board, -beta, -alpha, ply + 1, delta_prune, tick);
            board.undo_move();
            if score >= beta {
                return beta;
            }
            if score > alpha {
                alpha = score;
            }
        }
        return alpha;
    }

    // `stand_pat` (the static eval of not capturing) establishes a lower bound: a
    // side is never forced to capture, so if the static score already exceeds
    // `beta` we can cut immediately.
    let stand_pat = evaluate(board);
    if stand_pat >= beta {
        return beta;
    }
    if stand_pat > alpha {
        alpha = stand_pat;
    }
    if ply >= QS_MAX_PLY {
        return alpha;
    }

    let moves = board.generate_moves();
    for m in moves.iter() {
        if !m.is_capture() {
            continue;
        }

        // Delta pruning: if even winning the victim outright (plus a safety
        // margin) cannot lift stand-pat to alpha, the capture is pointless.
        // Promotions are exempt — their material swing exceeds the victim.
        if delta_prune && !m.is_promo() {
            let victim = board.piece_at(m.get_dest()).type_of();
            if stand_pat + PIECE_VALUE_MG[victim as usize] + 200 <= alpha {
                continue;
            }
        }

        // SEE pruning: never search (or play into) a capture that loses material on
        // the exchange. This is what stops the engine from grabbing a defended piece
        // with a more valuable one — the "trade a strong piece for a pawn" blunder.
        if see(board, *m) < 0 {
            continue;
        }

        board.apply_move(*m);
        let score = -quiesce(board, -beta, -alpha, ply + 1, delta_prune, tick);
        board.undo_move();

        if score >= beta {
            return beta;
        }
        if score > alpha {
            alpha = score;
        }
    }

    alpha
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checkmated_position_scores_as_mate_not_static_eval() {
        // Black king a12 mated by white queen b11 defended by white king c10.
        let mut board = Board::from_hfen("k11/1Q10/2K9/12/12/12/12/12/12/12/12/12 b - - 0 1")
            .expect("mate HFEN should parse");
        assert!(board.in_check());
        assert!(board.generate_moves().is_empty());
        let v = quiesce(
            &mut board,
            -VALUE_INFINITE,
            VALUE_INFINITE,
            4,
            false,
            &mut || false,
        );
        assert_eq!(v, -VALUE_MATE + 4);
    }

    #[test]
    fn in_check_has_no_stand_pat_cutoff() {
        // Black king a12 in check from queen c12 along the rank; black has escapes.
        // With naive stand-pat an in-check node could cut on a favourable static
        // eval; the evasion search must return a real (searched) value instead.
        let mut board = Board::from_hfen("k1Q9/12/2K9/12/12/12/12/12/12/12/12/12 b - - 0 1")
            .expect("check HFEN should parse");
        assert!(board.in_check());
        assert!(!board.generate_moves().is_empty());
        let v = quiesce(
            &mut board,
            -VALUE_INFINITE,
            VALUE_INFINITE,
            0,
            false,
            &mut || false,
        );
        // Down a queen but not mated: the score must be a normal (non-mate) value.
        assert!(!crate::search::is_mate_score(v), "got {v}");
    }

    #[test]
    fn tick_abort_is_honoured() {
        let mut board = Board::start_pos();
        let mut calls = 0u32;
        let _ = quiesce(
            &mut board,
            -VALUE_INFINITE,
            VALUE_INFINITE,
            0,
            false,
            &mut || {
                calls += 1;
                true // abort immediately
            },
        );
        assert_eq!(calls, 1, "aborted quiesce must not expand children");
    }
}
