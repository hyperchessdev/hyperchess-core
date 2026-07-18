//! Scoring for nodes that have no legal moves, plus transposition-table-safe
//! handling of mate scores.

use hyperchess_rules::board::Board;
use hyperchess_rules::core::score::*;

/// Any score with magnitude above this is treated as a mate score. Comfortably
/// above the largest reachable material/positional evaluation yet below the
/// smallest real mate score (`VALUE_MATE - max_ply`).
pub const MATE_BOUND: Value = VALUE_MATE - 2048;

/// Value of a position with no legal moves, from the side-to-move's perspective.
///
/// * In check  → checkmate. Scored as `-VALUE_MATE` offset by `ply` — the
///   distance from the **search root**, not the absolute game ply — so *faster*
///   mates are preferred and mate scores stay within `VALUE_MATE_IN_MAX_PLY`
///   regardless of how long the game has run.
/// * Not in check → stalemate, scored as a draw.
#[inline]
pub fn terminal_value(board: &Board, ply: Value) -> Value {
    if board.in_check() {
        -VALUE_MATE + ply
    } else {
        VALUE_DRAW
    }
}

/// True if `v` encodes a forced mate rather than a heuristic evaluation.
#[inline]
pub fn is_mate_score(v: Value) -> bool {
    v.abs() >= MATE_BOUND
}

/// Convert a search score into the form stored in the transposition table.
///
/// Mate scores here bake in the *absolute* ply of the mate; to make a TT entry
/// reusable across transpositions reached at different plies, we re-express the
/// mate score as a distance from the storing node by removing this node's ply.
/// Non-mate scores are unchanged. (Inverse of [`value_from_tt`].)
#[inline]
pub fn value_to_tt(v: Value, ply: Value) -> Value {
    if is_mate_score(v) {
        v + v.signum() * ply
    } else {
        v
    }
}

/// Re-project a TT score back to the probing node's absolute-ply mate scale.
#[inline]
pub fn value_from_tt(v: Value, ply: Value) -> Value {
    if is_mate_score(v) {
        v - v.signum() * ply
    } else {
        v
    }
}
