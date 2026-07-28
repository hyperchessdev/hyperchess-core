// SPDX-License-Identifier: GPL-3.0-or-later
// HyperChess Core — hyperchess-eval
// File: crates/hyperchess-eval/src/lib.rs
// Version: 1.0.0
// Copyright (c) 2026 HyperChess Developer Team

//! Single source of truth for HyperChess static-evaluation math.
//!
//! This crate is `#![no_std]` and uses only `core` integer arithmetic, so the
//! exact same functions are compiled into BOTH the CPU evaluator
//! (`hyperchess::tools::eval`) and the GPU kernel
//! (`hyperchess_kernels::eval`). That guarantees the GPU ranks moves against the
//! *identical* evaluation the CPU search uses — previously the two had drifted
//! (different phase constant, blend rounding, rook/king PSQT).
//!
//! All values are from White's perspective; callers mirror the rank for Black.
//! Piece types are the `PieceType` discriminants: 0=None 1=P 2=N 3=B 4=R 5=Q
//! 6=K 7=E 8=H 9=All.

#![no_std]

/// Material value (middlegame) indexed by piece-type discriminant.
pub const PIECE_VALUE_MG: [i32; 10] = [0, 100, 320, 330, 500, 900, 0, 700, 550, 0];
/// Material value (endgame) indexed by piece-type discriminant.
pub const PIECE_VALUE_EG: [i32; 10] = [0, 100, 320, 330, 500, 900, 0, 700, 550, 0];

/// Phase weight per piece-type discriminant. Heavier pieces keep the game in the
/// middlegame longer. Pawns and kings contribute 0.
pub const PHASE_WEIGHT: [i32; 10] = [0, 0, 1, 1, 2, 4, 0, 3, 2, 0];

/// Maximum phase weight present in the starting position
/// (4·N + 4·B + 4·R·2 + 2·Q·4 + 4·E·3 + 4·H·2 = 44).
pub const TOTAL_PHASE: i32 = 44;

/// Center proximity bonus: 22 at the four central squares, decreasing to 0 at
/// the corners of the 12×12 board.
#[inline]
pub fn center_bonus(rank: i32, file: i32) -> i32 {
    let cf = (file * 2 - 11).abs();
    let cr = (rank * 2 - 11).abs();
    22 - cf - cr
}

/// Small bonus for rooks on the four central files (e–h, indices 4–7).
#[inline]
pub fn fifth_file_bonus(file: i32) -> i32 {
    if (4..=7).contains(&file) {
        5
    } else {
        0
    }
}

/// King placement score (middlegame): rewards staying on the back ranks and
/// near the corners, penalises wandering up the board.
#[inline]
pub fn king_safety_score(rank: i32, file: i32) -> i32 {
    let back_rank_bonus = if rank <= 1 {
        20
    } else if rank <= 2 {
        10
    } else {
        -10 * rank
    };
    let corner_bonus = if file <= 2 || file >= 9 { 10 } else { -5 };
    back_rank_bonus + corner_bonus
}

/// Piece-square `(mg, eg)` bonus for a piece type at `(rank, file)` from White's
/// perspective. `rank`/`file` are 0-based; Black callers pass the mirrored rank.
#[inline]
pub fn raw_psqt(piece_type: i32, rank: i32, file: i32) -> (i32, i32) {
    let cb = center_bonus(rank, file);
    match piece_type {
        1 => {
            // Pawn — reward advancement.
            let adv = rank * 5;
            (cb / 2 + adv, adv + 10)
        }
        2 => (cb * 2, cb), // Knight — strongly likes the center
        3 => (cb + 5, cb), // Bishop
        4 => {
            // Rook — central files, relative 7th rank.
            let seventh = if rank == 9 { 10 } else { 0 };
            (fifth_file_bonus(file) + seventh, cb / 2)
        }
        5 => (cb, cb / 2),                            // Queen
        6 => (king_safety_score(rank, file), cb * 2), // King
        7 => (cb + 3, cb),                            // Eagle (rook-like)
        8 => (cb + 2, cb),                            // Hawk (bishop-like)
        _ => (0, 0),
    }
}

/// Convert a summed phase weight of the non-pawn material present on the board
/// into a blended phase in `[0, 256]` (256 = pure middlegame, 0 = pure endgame).
#[inline]
pub fn normalize_phase(present_weight_sum: i32) -> i32 {
    let phase = {
        let p = TOTAL_PHASE - present_weight_sum;
        if p < 0 {
            0
        } else {
            p
        }
    };
    let denom = if TOTAL_PHASE > 0 { TOTAL_PHASE } else { 1 };
    ((TOTAL_PHASE - phase) * 256 + TOTAL_PHASE / 2) / denom
}

/// Blend middlegame and endgame scores by `phase` (256 = MG, 0 = EG) using a
/// single rounded division (matches the historical CPU evaluator exactly).
#[inline]
pub fn blend(mg: i32, eg: i32, phase: i32) -> i32 {
    ((mg as i64 * phase as i64 + eg as i64 * (256 - phase as i64)) / 256) as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn center_bonus_peaks_at_center_and_zeros_at_corners() {
        // Most central squares (rank/file 5 or 6) → 22 - 1 - 1 = 20.
        assert_eq!(center_bonus(5, 5), 20);
        assert_eq!(center_bonus(6, 6), 20);
        assert_eq!(center_bonus(5, 6), 20);
        // Corners → 0, and it is symmetric across the board.
        assert_eq!(center_bonus(0, 0), 0);
        assert_eq!(center_bonus(11, 11), 0);
        assert_eq!(center_bonus(0, 11), 0);
        assert_eq!(center_bonus(11, 0), 0);
    }

    #[test]
    fn fifth_file_bonus_only_central_files() {
        for f in 4..=7 {
            assert_eq!(fifth_file_bonus(f), 5, "file {f} is central");
        }
        for f in [0, 1, 2, 3, 8, 9, 10, 11] {
            assert_eq!(fifth_file_bonus(f), 0, "file {f} is not central");
        }
    }

    #[test]
    fn king_safety_rewards_back_rank_corners() {
        assert_eq!(king_safety_score(0, 0), 30); // back rank (20) + corner (10)
        assert_eq!(king_safety_score(0, 9), 30); // other-side corner
        assert_eq!(king_safety_score(2, 5), 5); // rank 2 (10) + centre file (-5)
        assert_eq!(king_safety_score(5, 5), -55); // wandered up (-50) + centre (-5)
    }

    #[test]
    fn raw_psqt_matches_hand_computed() {
        // Pawn at (rank 2, file 5): cb=14, adv=10 -> (14/2+10, 10+10).
        assert_eq!(raw_psqt(1, 2, 5), (17, 20));
        // Knight at centre: cb=20 -> (40, 20).
        assert_eq!(raw_psqt(2, 5, 5), (40, 20));
        // King at corner: (king_safety(30), cb*2=0).
        assert_eq!(raw_psqt(6, 0, 0), (30, 0));
        // Unknown piece type -> zero.
        assert_eq!(raw_psqt(9, 5, 5), (0, 0));
    }

    #[test]
    fn phase_endpoints() {
        // Full non-pawn material present -> pure middlegame (256).
        assert_eq!(normalize_phase(TOTAL_PHASE), 256);
        // No material -> pure endgame (0).
        assert_eq!(normalize_phase(0), 0);
        // Monotonic between the endpoints.
        assert!(normalize_phase(TOTAL_PHASE / 2) > 0);
        assert!(normalize_phase(TOTAL_PHASE / 2) < 256);
    }

    #[test]
    fn blend_interpolates_mg_eg() {
        assert_eq!(blend(100, 50, 256), 100); // pure MG
        assert_eq!(blend(100, 50, 0), 50); // pure EG
        assert_eq!(blend(100, 50, 128), 75); // halfway
    }
}
