//! GPU batch position evaluation kernel.
//!
//! The evaluation MATH (material, PSQT, phase, blend) lives in the shared
//! `hyperchess_eval` crate and is the *same* code the CPU evaluator
//! (`hyperchess_rules::tools::eval`) uses, so GPU and CPU scores are byte-identical.
//! This file only handles board decoding and the parallel kernel launch.
//!
//! Board encoding (Piece discriminants, must match hyperchess_rules::core::Piece):
//!   0=None
//!   1=WhitePawn   2=WhiteKnight  3=WhiteBishop  4=WhiteRook
//!   5=WhiteQueen  6=WhiteKing    7=WhiteEagle    8=WhiteHawk
//!   9=BlackPawn  10=BlackKnight 11=BlackBishop  12=BlackRook
//!  13=BlackQueen 14=BlackKing   15=BlackEagle   16=BlackHawk

use cuda_std::prelude::*;
use hyperchess_eval::{blend, normalize_phase, raw_psqt, PHASE_WEIGHT, PIECE_VALUE_EG, PIECE_VALUE_MG};

/// Given a Piece discriminant (0-16), return its PieceType (0-8).
#[inline]
fn piece_type_of(p: u8) -> i32 {
    match p {
        0 => 0,
        1..=8 => p as i32,        // white pieces: discriminant == piece_type
        9..=16 => (p - 8) as i32, // black pieces: subtract 8
        _ => 0,
    }
}

/// Given a Piece discriminant, return true if it is White.
#[inline]
fn is_white(p: u8) -> bool {
    p >= 1 && p <= 8
}

/// Compute game phase from the piece discriminants in the board slice.
#[inline]
fn compute_phase(board: &[u8]) -> i32 {
    let mut present = 0i32;
    for &p in board.iter() {
        present += PHASE_WEIGHT[piece_type_of(p) as usize];
    }
    normalize_phase(present)
}

/// Evaluate a single 144-square board slice.
/// Returns score from White's perspective (positive = White ahead).
#[inline]
fn eval_one(board: &[u8]) -> i32 {
    let phase = compute_phase(board);
    let mut score_mg: i32 = 0;
    let mut score_eg: i32 = 0;

    for sq in 0..144usize {
        let p = board[sq];
        if p == 0 {
            continue;
        }
        let pt = piece_type_of(p);
        let rank = (sq / 12) as i32;
        let file = (sq % 12) as i32;

        // For Black, mirror rank for PSQT
        let (psqt_rank, psqt_file) = if is_white(p) {
            (rank, file)
        } else {
            (11 - rank, file)
        };

        let (psqt_mg, psqt_eg) = raw_psqt(pt, psqt_rank, psqt_file);
        let mat_mg = PIECE_VALUE_MG[pt as usize];
        let mat_eg = PIECE_VALUE_EG[pt as usize];

        if is_white(p) {
            score_mg += mat_mg + psqt_mg;
            score_eg += mat_eg + psqt_eg;
        } else {
            score_mg -= mat_mg + psqt_mg;
            score_eg -= mat_eg + psqt_eg;
        }
    }

    blend(score_mg, score_eg, phase)
}

// ── GPU Kernel ────────────────────────────────────────────────────────────────

/// Evaluate N board positions in parallel.
///
/// Parameters:
///   boards         - packed board arrays: n * 144 bytes, one u8 per square (Piece discriminant)
///   side_to_move   - one byte per board: 0=White to move, 1=Black to move
///   scores         - output: one i32 per board (from the side-to-move's perspective)
///   n              - number of boards
#[kernel]
#[allow(improper_ctypes_definitions)]
pub unsafe fn batch_eval(
    boards: &[u8],
    side_to_move: &[u8],
    scores: *mut i32,
    n: u32,
) {
    let idx = thread::index_1d() as usize;
    if idx >= n as usize {
        return;
    }

    let base = idx * 144;
    let board_slice = &boards[base..base + 144];
    let white_abs = eval_one(board_slice);

    // Negate if Black to move (match CPU evaluate() behaviour)
    let score = if side_to_move[idx] == 0 { white_abs } else { -white_abs };

    unsafe {
        *scores.add(idx) = score;
    }
}
