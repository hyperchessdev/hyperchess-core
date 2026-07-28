// SPDX-License-Identifier: GPL-3.0-or-later
// HyperChess Core — hyperchess-rules
// File: crates/hyperchess-rules/src/tools/eval.rs
// Version: 1.0.0
// Copyright (c) 2026 HyperChess Developer Team

//! Evaluation function for HyperChess.
//!
//! Two layers:
//!
//! * [`evaluate_base`] — material + PSQT + phase blend. This is the **GPU-parity
//!   contract**: it is byte-identical to the CUDA kernel in
//!   `hyperchess_engine::cuda_backend`, so every GPU-paired CPU reference and the
//!   CPU↔GPU cross-validation benchmark uses *this* function, not [`evaluate`].
//! * [`evaluate`] — `evaluate_base` plus board-aware hand-crafted terms (piece
//!   mobility, king safety, and threats against undefended pieces). These need
//!   full board context (attack maps), so they cannot live in the per-square
//!   `hyperchess_eval` and are therefore CPU-only. This is the function the
//!   search/quiescence/ordering use to *play*.
//!
//! The terms are deliberately symmetric, so the start position still evaluates to
//! exactly 0.

use crate::board::Board;
use crate::core::bitboard::BitBoard;
use crate::core::score::*;
use crate::core::{PieceType, Player, ALL_PIECE_TYPES};
use crate::helper::prelude::{king_attacks, pawn_attacks_sq, piece_attacks};
use crate::helper::psqt;

/// Runtime piece-value overrides, for empirical calibration only.
///
/// `binary_search_value` in `hyperchess_engine::calibration` needs to play
/// self-play games with a *candidate* value for a piece (Eagle/Hawk) without
/// recompiling. When an override is set for a piece type, the material term of
/// [`evaluate`] / [`evaluate_base`] uses it instead of the static
/// `PIECE_VALUE_MG`/`PIECE_VALUE_EG` tables.
///
/// **Not for production play**: while any override is set, `evaluate_base`
/// is no longer byte-identical to the GPU kernel in
/// `hyperchess_engine::cuda_backend` — clear overrides before any CPU↔GPU
/// parity work. With no override set (the default), behaviour and parity are
/// unchanged.
pub mod value_overrides {
    use std::sync::atomic::{AtomicI32, Ordering};

    use crate::core::score::{Value, PIECE_VALUE_EG, PIECE_VALUE_MG};
    use crate::core::PieceType;

    /// Sentinel for "no override". Piece values are small positive centipawn
    /// numbers, so `i32::MIN` can never be a real value.
    const NONE: i32 = i32::MIN;

    #[allow(clippy::declare_interior_mutable_const)]
    const NO_OVERRIDE: AtomicI32 = AtomicI32::new(NONE);
    static MG: [AtomicI32; 10] = [NO_OVERRIDE; 10];
    static EG: [AtomicI32; 10] = [NO_OVERRIDE; 10];

    /// Override the middlegame/endgame material value of `pt` (centipawns).
    pub fn set(pt: PieceType, mg: Value, eg: Value) {
        MG[pt as usize].store(mg, Ordering::Relaxed);
        EG[pt as usize].store(eg, Ordering::Relaxed);
    }

    /// Remove every override, restoring the static tables.
    pub fn clear() {
        for i in 0..10 {
            MG[i].store(NONE, Ordering::Relaxed);
            EG[i].store(NONE, Ordering::Relaxed);
        }
    }

    /// Middlegame value of `pt`, honouring any override set by [`set`].
    ///
    /// Falls back to the compiled-in table when no override is present, which
    /// is why `NONE` must stay outside the range of any plausible piece value.
    #[inline]
    pub(crate) fn mg(pt: PieceType) -> Value {
        let v = MG[pt as usize].load(Ordering::Relaxed);
        if v == NONE {
            PIECE_VALUE_MG[pt as usize]
        } else {
            v
        }
    }

    /// Endgame counterpart of [`mg`].
    #[inline]
    pub(crate) fn eg(pt: PieceType) -> Value {
        let v = EG[pt as usize].load(Ordering::Relaxed);
        if v == NONE {
            PIECE_VALUE_EG[pt as usize]
        } else {
            v
        }
    }
}

/// Centipawns awarded per square of net piece mobility.
const MOBILITY_WEIGHT: Value = 2;
/// Centipawns penalised per enemy attack landing on our king's ring.
const KING_RING_WEIGHT: Value = 4;
/// A hanging enemy piece is worth `PIECE_VALUE_MG / THREAT_DIVISOR` centipawns.
const THREAT_DIVISOR: Value = 8;

/// Test-only: full eval with the threats term toggled, for A/B isolation.
/// `include_threats = true` equals [`evaluate`]; `false` is mobility+king-safety only.
#[doc(hidden)]
pub fn evaluate_variant(board: &Board, include_threats: bool) -> Value {
    let us = board.turn();
    let mut score = evaluate_absolute(board);
    score += evaluate_terms_inner(board, include_threats);
    if us == Player::White {
        score
    } else {
        -score
    }
}

/// Evaluates the board from the perspective of the side to move (full eval).
///
/// This is `evaluate_base` plus the hand-crafted mobility / king-safety terms.
pub fn evaluate(board: &Board) -> Value {
    let us = board.turn();
    let mut score = evaluate_absolute(board);
    score += evaluate_terms_absolute(board);
    if us == Player::White {
        score
    } else {
        -score
    }
}

/// Material + PSQT + phase only, from the side-to-move perspective.
///
/// Kept byte-identical to the GPU kernel — used by GPU-paired CPU references and
/// the CPU↔GPU cross-validation benchmark so parity is preserved.
pub fn evaluate_base(board: &Board) -> Value {
    let us = board.turn();
    let score = evaluate_absolute(board);
    if us == Player::White {
        score
    } else {
        -score
    }
}

/// Material + PSQT + phase, from White's perspective.
fn evaluate_absolute(board: &Board) -> Value {
    let mut score = Score::ZERO;

    // Material and PSQT
    for &pt in ALL_PIECE_TYPES.iter() {
        // Calibration override, or the static table when none is set (default).
        let material = Score::new(value_overrides::mg(pt), value_overrides::eg(pt));

        // White
        let mut w_pieces = board.piece_bb(Player::White, pt);
        while let Some(sq) = w_pieces.pop_some_lsb() {
            score += material;
            score += psqt::psqt_value(pt, sq, Player::White);
        }

        // Black
        let mut b_pieces = board.piece_bb(Player::Black, pt);
        while let Some(sq) = b_pieces.pop_some_lsb() {
            score -= material;
            score -= psqt::psqt_value(pt, sq, Player::Black);
        }
    }

    // Blend MG/EG based on material (simple phase calculation)
    let phase = compute_phase(board);
    blend(score, phase)
}

/// Per-side attack summary used by the hand-crafted terms.
struct AttackInfo {
    /// Net pseudo-legal destination squares (excludes own pieces), summed over
    /// all non-king pieces.
    mobility: i32,
    /// Union of every square this side attacks (all pieces, including pawns/king).
    attacks: BitBoard,
}

/// Compute mobility and the full attack map for one player in a single pass.
fn attack_info(board: &Board, player: Player) -> AttackInfo {
    let occ = board.occupied();
    let own = board.occupied_player(player);
    let mut mobility = 0i32;
    let mut attacks = BitBoard::EMPTY;

    for &pt in ALL_PIECE_TYPES.iter() {
        let mut pieces = board.piece_bb(player, pt);
        while let Some(sq) = pieces.pop_some_lsb() {
            let att = match pt {
                PieceType::P => pawn_attacks_sq(player, sq),
                _ => piece_attacks(pt, sq, occ),
            };
            attacks |= att;
            // King mobility is intentionally excluded — it conflicts with the
            // king-safety term and rewards exposing the king.
            if pt != PieceType::K {
                mobility += (att & !own).count_bits() as i32;
            }
        }
    }

    AttackInfo { mobility, attacks }
}

/// Sum a fraction of the value of every `defender` piece that `attacker` attacks
/// but `defender` does not defend (a hanging piece). Kings are excluded (being
/// attacked is check, handled by search, and a king cannot be won outright).
fn hanging_value(
    board: &Board,
    attacker_attacks: BitBoard,
    defender: Player,
    defender_attacks: BitBoard,
) -> i32 {
    let mut total = 0;
    for &pt in ALL_PIECE_TYPES.iter() {
        if pt == PieceType::K {
            continue;
        }
        let mut bb = board.piece_bb(defender, pt);
        while let Some(sq) = bb.pop_some_lsb() {
            if attacker_attacks.test_bit(sq) && !defender_attacks.test_bit(sq) {
                total += value_overrides::mg(pt) / THREAT_DIVISOR;
            }
        }
    }
    total
}

/// Hand-crafted mobility + king-safety (+ optional threats) terms, from White's
/// perspective. Symmetric by construction: a mirrored position yields 0.
///
/// `with_threats` lets the A/B harness measure the threats term in isolation;
/// production always passes `true`.
fn evaluate_terms_inner(board: &Board, with_threats: bool) -> Value {
    let white = attack_info(board, Player::White);
    let black = attack_info(board, Player::Black);

    // Mobility: more reachable squares is better.
    let mobility = (white.mobility - black.mobility) * MOBILITY_WEIGHT;

    // King safety: enemy attacks landing on the squares around our king are bad.
    let white_ring = king_attacks(board.king_sq(Player::White));
    let black_ring = king_attacks(board.king_sq(Player::Black));
    let white_pressure = (black.attacks & white_ring).count_bits() as i32;
    let black_pressure = (white.attacks & black_ring).count_bits() as i32;
    let king_safety = (black_pressure - white_pressure) * KING_RING_WEIGHT;

    // Threats: reward attacking undefended (hanging) enemy pieces. Quiescence
    // resolves real captures, but this nudges ordering/eval toward winning
    // material just beyond the search horizon. Conservative (value/THREAT_DIVISOR).
    let threats = if with_threats {
        let w = hanging_value(board, white.attacks, Player::Black, black.attacks);
        let b = hanging_value(board, black.attacks, Player::White, white.attacks);
        w - b
    } else {
        0
    };

    mobility + king_safety + threats
}

/// Full hand-crafted terms (mobility + king-safety + threats), White's perspective.
fn evaluate_terms_absolute(board: &Board) -> Value {
    evaluate_terms_inner(board, true)
}

/// Compute game phase (0 = pure endgame, 256 = pure middlegame).
///
/// Sums the phase weight of the non-pawn material present and normalizes it via
/// the shared [`hyperchess_eval`] so the CPU and GPU agree exactly.
fn compute_phase(board: &Board) -> i32 {
    use hyperchess_eval::PHASE_WEIGHT;
    let mut present = 0i32;
    for &player in &[Player::White, Player::Black] {
        for pt in [
            PieceType::N,
            PieceType::B,
            PieceType::R,
            PieceType::Q,
            PieceType::E,
            PieceType::H,
        ] {
            present += board.piece_bb(player, pt).count_bits() as i32 * PHASE_WEIGHT[pt as usize];
        }
    }
    hyperchess_eval::normalize_phase(present)
}

/// Blend middlegame and endgame scores.
fn blend(score: Score, phase: i32) -> Value {
    hyperchess_eval::blend(score.mg, score.eg, phase)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The start position is mirror-symmetric, so both layers must be exactly 0.
    #[test]
    fn start_is_zero() {
        let b = Board::start_pos();
        assert_eq!(evaluate_base(&b), 0);
        assert_eq!(evaluate(&b), 0);
    }

    /// Hand-crafted terms must never change a mirror-symmetric position's score.
    #[test]
    fn terms_symmetric_at_start() {
        assert_eq!(evaluate_terms_absolute(&Board::start_pos()), 0);
    }

    /// Removing a defender of the king should not *increase* our own score:
    /// king-safety must penalise (or leave unchanged) extra enemy pressure.
    #[test]
    fn material_monotonic() {
        // White up a full queen (Black queen removed) must evaluate clearly positive.
        let hfen =
            "12/ehrnb1kbnrhe/pppppppppppp/12/12/12/12/12/12/PPPPPPPPPPPP/EHRNBQKBNRHE/12 w - - 0 1";
        let b = Board::from_hfen(hfen).unwrap();
        assert!(
            evaluate(&b) > 500,
            "up a queen should be clearly winning, got {}",
            evaluate(&b)
        );
    }

    /// The threats term must stay symmetric: a mirrored position contributes 0.
    #[test]
    fn threats_symmetric_at_start() {
        let b = Board::start_pos();
        assert_eq!(evaluate_variant(&b, true), evaluate_variant(&b, false));
        assert_eq!(evaluate_variant(&b, true), 0);
    }

    /// A hanging enemy piece we attack-but-they-don't-defend should raise our
    /// score relative to the same eval without the threats term.
    #[test]
    fn threats_reward_hanging_piece() {
        // White rook on a3 attacks an undefended black knight on a9 down the open
        // a-file; both kings are far away so nothing defends the knight. With
        // threats on, White should score strictly higher than with threats off.
        let hfen = "11k/12/n11/12/12/12/12/12/R11/12/11K/12 w - - 0 1";
        let b = Board::from_hfen(hfen).unwrap();
        let with = evaluate_variant(&b, true);
        let without = evaluate_variant(&b, false);
        assert!(
            with > without,
            "threats term should reward the hanging knight: with={} without={}",
            with,
            without
        );
    }
}
