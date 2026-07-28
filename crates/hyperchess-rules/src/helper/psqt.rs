// SPDX-License-Identifier: GPL-3.0-or-later
// HyperChess Core — hyperchess-rules
// File: crates/hyperchess-rules/src/helper/psqt.rs
// Version: 1.0.0
// Copyright (c) 2026 HyperChess Developer Team

//! Piece-square tables for 12x12 HyperChess.
//!
//! Values are from White's perspective. For Black, mirror the rank.

use crate::core::masks::*;
use crate::core::score::{Score, Value};
use crate::core::sq::SQ;
use crate::core::{PieceType, Player};

/// PSQT values indexed by [piece_type][square]. Piece types 0 and 9 are unused.
pub static mut PSQT: [[Score; SQ_CNT]; PIECE_TYPE_CNT] = [[Score::ZERO; SQ_CNT]; PIECE_TYPE_CNT];

/// Initializes PSQT tables.
pub fn init_psqt() {
    for pt_idx in 1..=8usize {
        for sq in 0..144usize {
            let rank = sq / 12;
            let file = sq % 12;
            let score = raw_psqt_value(pt_idx, rank, file);
            unsafe {
                PSQT[pt_idx][sq] = score;
            }
        }
    }
}

/// Returns the PSQT score for a piece on a square (from White's perspective).
/// For Black, mirror the square.
#[inline(always)]
pub fn psqt_value(piece_type: PieceType, sq: SQ, player: Player) -> Score {
    let idx = match player {
        Player::White => sq.0 as usize,
        Player::Black => {
            // Mirror: rank' = 11 - rank, file stays
            let rank = sq.rank_idx();
            let file = sq.file_idx();
            ((11 - rank) * 12 + file) as usize
        }
    };
    unsafe { PSQT[piece_type as usize][idx] }
}

/// Basic PSQT value generation. Returns Score(mg, eg).
///
/// Delegates to [`hyperchess_eval::raw_psqt`] so the CPU evaluator and the
/// GPU kernel compute byte-identical piece-square values.
fn raw_psqt_value(piece_type: usize, rank: usize, file: usize) -> Score {
    let (mg, eg) = hyperchess_eval::raw_psqt(piece_type as i32, rank as i32, file as i32);
    Score::new(mg as Value, eg as Value)
}
