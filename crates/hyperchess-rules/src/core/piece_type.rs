//! The `PieceType` enum (kind of piece, side-agnostic) and move-generation kinds.

use std::fmt;

/// All real piece types (excluding None and All).
pub const ALL_PIECE_TYPES: [PieceType; 8] = [
    PieceType::P,
    PieceType::N,
    PieceType::B,
    PieceType::R,
    PieceType::Q,
    PieceType::K,
    PieceType::E,
    PieceType::H,
];

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum GenTypes {
    All,
    Captures,
    Quiets,
    QuietChecks,
    Evasions,
    NonEvasions,
}

#[repr(u8)]
#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
pub enum PieceType {
    None = 0,
    P = 1,
    N = 2,
    B = 3,
    R = 4,
    Q = 5,
    K = 6,
    E = 7, // Eagle
    H = 8, // Hawk
    All = 9,
}

impl PieceType {
    /// Returns the relative material value (for MVV-LVA ordering).
    #[inline]
    pub fn value(self) -> i16 {
        match self {
            PieceType::P => 1,
            PieceType::N | PieceType::B => 3,
            PieceType::R => 5,
            PieceType::H => 5,
            PieceType::E => 7,
            PieceType::Q => 9,
            _ => 0,
        }
    }

    #[inline(always)]
    pub fn is_none(self) -> bool {
        self == PieceType::None
    }

    #[inline(always)]
    pub fn is_some(self) -> bool {
        !self.is_none()
    }

    #[inline(always)]
    pub fn is_real(self) -> bool {
        self != PieceType::None && self != PieceType::All
    }

    /// Returns lowercase character for HFEN notation.
    #[inline]
    pub fn char_lower(self) -> char {
        match self {
            PieceType::P => 'p',
            PieceType::N => 'n',
            PieceType::B => 'b',
            PieceType::R => 'r',
            PieceType::Q => 'q',
            PieceType::K => 'k',
            PieceType::E => 'e',
            PieceType::H => 'h',
            _ => '?',
        }
    }

    /// Returns uppercase character for HFEN notation.
    #[inline]
    pub fn char_upper(self) -> char {
        match self {
            PieceType::P => 'P',
            PieceType::N => 'N',
            PieceType::B => 'B',
            PieceType::R => 'R',
            PieceType::Q => 'Q',
            PieceType::K => 'K',
            PieceType::E => 'E',
            PieceType::H => 'H',
            _ => '?',
        }
    }

    /// Parse from a character.
    pub fn from_char(c: char) -> Option<PieceType> {
        match c.to_ascii_lowercase() {
            'p' => Some(PieceType::P),
            'n' => Some(PieceType::N),
            'b' => Some(PieceType::B),
            'r' => Some(PieceType::R),
            'q' => Some(PieceType::Q),
            'k' => Some(PieceType::K),
            'e' => Some(PieceType::E),
            'h' => Some(PieceType::H),
            _ => None,
        }
    }

    /// Index for array access.
    #[inline(always)]
    pub fn index(self) -> usize {
        self as usize
    }

    /// Promotion piece types.
    pub const PROMO_TYPES: [PieceType; 6] = [
        PieceType::Q,
        PieceType::R,
        PieceType::B,
        PieceType::N,
        PieceType::E,
        PieceType::H,
    ];
}

impl fmt::Display for PieceType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = match *self {
            PieceType::P => "Pawn",
            PieceType::N => "Knight",
            PieceType::B => "Bishop",
            PieceType::R => "Rook",
            PieceType::Q => "Queen",
            PieceType::K => "King",
            PieceType::E => "Eagle",
            PieceType::H => "Hawk",
            PieceType::All => "All",
            PieceType::None => "",
        };
        f.pad(s)
    }
}
