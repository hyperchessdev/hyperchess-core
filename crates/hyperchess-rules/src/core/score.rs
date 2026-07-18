//! Score and Value types for the engine.

use std::fmt;
use std::ops::{Add, AddAssign, Mul, Neg, Sub, SubAssign};

/// Type alias for evaluation values (centipawns).
pub type Value = i32;

/// Known evaluation constants.
pub const VALUE_ZERO: Value = 0;
pub const VALUE_DRAW: Value = 0;
pub const VALUE_KNOWN_WIN: Value = 10000;
pub const VALUE_MATE: Value = 32000;
pub const VALUE_INFINITE: Value = 32001;
pub const VALUE_NONE: Value = 32002;

pub const VALUE_MATE_IN_MAX_PLY: Value = VALUE_MATE - 256;
pub const VALUE_MATED_IN_MAX_PLY: Value = -VALUE_MATE + 256;

/// Material values for each piece type.
pub const PAWN_VALUE_MG: Value = 100;
pub const KNIGHT_VALUE_MG: Value = 320;
pub const BISHOP_VALUE_MG: Value = 330;
pub const ROOK_VALUE_MG: Value = 500;
pub const QUEEN_VALUE_MG: Value = 900;
pub const EAGLE_VALUE_MG: Value = 700;
pub const HAWK_VALUE_MG: Value = 550;

pub const PAWN_VALUE_EG: Value = 100;
pub const KNIGHT_VALUE_EG: Value = 320;
pub const BISHOP_VALUE_EG: Value = 330;
pub const ROOK_VALUE_EG: Value = 500;
pub const QUEEN_VALUE_EG: Value = 900;
pub const EAGLE_VALUE_EG: Value = 700;
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
    pub mg: Value,
    pub eg: Value,
}

impl Score {
    pub const ZERO: Score = Score { mg: 0, eg: 0 };

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
