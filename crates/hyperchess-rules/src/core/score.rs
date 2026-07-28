// SPDX-License-Identifier: GPL-3.0-or-later
// HyperChess Core — hyperchess-rules
// File: crates/hyperchess-rules/src/core/score.rs
// Version: 1.0.0
// Copyright (c) 2026 HyperChess Developer Team

//! Score and Value types for the engine.

use std::fmt;
use std::ops::{Add, AddAssign, Mul, Neg, Sub, SubAssign};

/// Type alias for evaluation values (centipawns).
pub type Value = i32;

/// Known evaluation constants.
pub const VALUE_ZERO: Value = 0;
/// A drawn position. Identical to [`VALUE_ZERO`]; kept distinct so eval code
/// can say which of the two it means.
pub const VALUE_DRAW: Value = 0;
/// Threshold above which a position is considered winning on material/technique
/// alone, without a mate score being proven.
pub const VALUE_KNOWN_WIN: Value = 10000;
/// A mate at the root. Mate scores are stored as `VALUE_MATE - ply`, so a
/// shorter mate always outranks a longer one.
pub const VALUE_MATE: Value = 32000;
/// Search window bound, one above [`VALUE_MATE`] so an alpha-beta window opened
/// at `±VALUE_INFINITE` can never clip a legitimate mate score.
pub const VALUE_INFINITE: Value = 32001;
/// "No score recorded" sentinel — deliberately outside the `±VALUE_INFINITE`
/// window so it cannot be confused with any real evaluation.
pub const VALUE_NONE: Value = 32002;

/// Lowest score still recognised as a mate for the side to move; anything at or
/// above this is `VALUE_MATE - ply` for some ply below the 256 search cap.
pub const VALUE_MATE_IN_MAX_PLY: Value = VALUE_MATE - 256;
/// Mirror of [`VALUE_MATE_IN_MAX_PLY`] for being mated.
pub const VALUE_MATED_IN_MAX_PLY: Value = -VALUE_MATE + 256;

/// Material values for each piece type.
pub const PAWN_VALUE_MG: Value = 100;
/// Knight, middlegame.
pub const KNIGHT_VALUE_MG: Value = 320;
/// Bishop, middlegame — a shade above the knight, the usual convention.
pub const BISHOP_VALUE_MG: Value = 330;
/// Rook, middlegame.
pub const ROOK_VALUE_MG: Value = 500;
/// Queen, middlegame.
pub const QUEEN_VALUE_MG: Value = 900;
/// Eagle, middlegame. HyperChess-original piece, valued between rook and queen.
pub const EAGLE_VALUE_MG: Value = 700;
/// Hawk, middlegame. HyperChess-original piece, valued just above a rook.
pub const HAWK_VALUE_MG: Value = 550;

/// Endgame material values. Currently identical to their middlegame
/// counterparts — the phase split exists in the [`Score`] type and is exercised
/// by the piece-square tables, but material itself is not yet phase-dependent.
pub const PAWN_VALUE_EG: Value = 100;
/// Knight, endgame. See [`PAWN_VALUE_EG`].
pub const KNIGHT_VALUE_EG: Value = 320;
/// Bishop, endgame. See [`PAWN_VALUE_EG`].
pub const BISHOP_VALUE_EG: Value = 330;
/// Rook, endgame. See [`PAWN_VALUE_EG`].
pub const ROOK_VALUE_EG: Value = 500;
/// Queen, endgame. See [`PAWN_VALUE_EG`].
pub const QUEEN_VALUE_EG: Value = 900;
/// Eagle, endgame. See [`PAWN_VALUE_EG`].
pub const EAGLE_VALUE_EG: Value = 700;
/// Hawk, endgame. See [`PAWN_VALUE_EG`].
pub const HAWK_VALUE_EG: Value = 550;

/// Piece values indexed by PieceType (0=None..8=Hawk, 9=All).
pub static PIECE_VALUE_MG: [Value; 10] = [
    0,               // None
    PAWN_VALUE_MG,   // P
    KNIGHT_VALUE_MG, // N
    BISHOP_VALUE_MG, // B
    ROOK_VALUE_MG,   // R
    QUEEN_VALUE_MG,  // Q
    0,               // K (no material value)
    EAGLE_VALUE_MG,  // E
    HAWK_VALUE_MG,   // H
    0,               // All
];

/// Endgame counterpart of [`PIECE_VALUE_MG`], same indexing.
pub static PIECE_VALUE_EG: [Value; 10] = [
    0,
    PAWN_VALUE_EG,
    KNIGHT_VALUE_EG,
    BISHOP_VALUE_EG,
    ROOK_VALUE_EG,
    QUEEN_VALUE_EG,
    0,
    EAGLE_VALUE_EG,
    HAWK_VALUE_EG,
    0,
];

/// A score combines middlegame and endgame values.
#[derive(Copy, Clone, PartialEq, Eq, Default)]
pub struct Score {
    /// Middlegame component, in centipawns.
    pub mg: Value,
    /// Endgame component, in centipawns.
    pub eg: Value,
}

impl Score {
    /// Additive identity — the starting accumulator for a term-by-term eval.
    pub const ZERO: Score = Score { mg: 0, eg: 0 };

    /// Pair a middlegame and an endgame value into one score.
    #[inline]
    pub const fn new(mg: Value, eg: Value) -> Self {
        Score { mg, eg }
    }
}

impl Add for Score {
    type Output = Score;
    #[inline]
    fn add(self, rhs: Score) -> Score {
        Score::new(self.mg + rhs.mg, self.eg + rhs.eg)
    }
}

impl AddAssign for Score {
    #[inline]
    fn add_assign(&mut self, rhs: Score) {
        self.mg += rhs.mg;
        self.eg += rhs.eg;
    }
}

impl Sub for Score {
    type Output = Score;
    #[inline]
    fn sub(self, rhs: Score) -> Score {
        Score::new(self.mg - rhs.mg, self.eg - rhs.eg)
    }
}

impl SubAssign for Score {
    #[inline]
    fn sub_assign(&mut self, rhs: Score) {
        self.mg -= rhs.mg;
        self.eg -= rhs.eg;
    }
}

impl Neg for Score {
    type Output = Score;
    #[inline]
    fn neg(self) -> Score {
        Score::new(-self.mg, -self.eg)
    }
}

impl Mul<i32> for Score {
    type Output = Score;
    #[inline]
    fn mul(self, rhs: i32) -> Score {
        Score::new(self.mg * rhs, self.eg * rhs)
    }
}

impl fmt::Debug for Score {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Score(mg={}, eg={})", self.mg, self.eg)
    }
}

impl fmt::Display for Score {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "({}, {})", self.mg, self.eg)
    }
}
