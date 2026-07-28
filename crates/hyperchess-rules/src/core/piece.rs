// SPDX-License-Identifier: GPL-3.0-or-later
// HyperChess Core — hyperchess-rules
// File: crates/hyperchess-rules/src/core/piece.rs
// Version: 1.0.0
// Copyright (c) 2026 HyperChess Developer Team

//! The `Piece` enum (player + piece type combined) and its HFEN mapping.

use std::fmt;

use super::piece_type::PieceType;
use super::player::Player;

/// Total number of unique Piece enum values (excluding None).
pub const PIECE_ENUM_CNT: usize = 17;

#[repr(u8)]
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
/// A piece together with its owner, as stored in the board's square array.
///
/// Discriminants are laid out so White's pieces occupy 1-8 and Black's repeat
/// the same order at 9-16. That regularity is what lets [`Piece::make_lossy`]
/// index a flat table by `player * 8 + piece_type` instead of branching.
pub enum Piece {
    /// Empty square.
    None = 0,
    WhitePawn = 1,
    WhiteKnight = 2,
    WhiteBishop = 3,
    WhiteRook = 4,
    WhiteQueen = 5,
    WhiteKing = 6,
    WhiteEagle = 7,
    WhiteHawk = 8,
    BlackPawn = 9,
    BlackKnight = 10,
    BlackBishop = 11,
    BlackRook = 12,
    BlackQueen = 13,
    BlackKing = 14,
    BlackEagle = 15,
    BlackHawk = 16,
}

impl Piece {
    /// Creates a Piece from Player and PieceType.
    #[inline]
    pub fn make(player: Player, piece_type: PieceType) -> Option<Piece> {
        if piece_type == PieceType::None || piece_type == PieceType::All {
            if piece_type == PieceType::None {
                return Some(Piece::None);
            }
            return None;
        }
        Some(Piece::make_lossy(player, piece_type))
    }

    /// Creates a Piece from Player and PieceType without checks.
    #[inline(always)]
    pub fn make_lossy(player: Player, piece_type: PieceType) -> Piece {
        let idx = player as u8 * 8 + piece_type as u8;
        ALL_PIECES_FLAT[idx as usize]
    }

    /// Returns the PieceType.
    #[inline(always)]
    pub fn type_of(self) -> PieceType {
        match self {
            Piece::None => PieceType::None,
            Piece::WhitePawn | Piece::BlackPawn => PieceType::P,
            Piece::WhiteKnight | Piece::BlackKnight => PieceType::N,
            Piece::WhiteBishop | Piece::BlackBishop => PieceType::B,
            Piece::WhiteRook | Piece::BlackRook => PieceType::R,
            Piece::WhiteQueen | Piece::BlackQueen => PieceType::Q,
            Piece::WhiteKing | Piece::BlackKing => PieceType::K,
            Piece::WhiteEagle | Piece::BlackEagle => PieceType::E,
            Piece::WhiteHawk | Piece::BlackHawk => PieceType::H,
        }
    }

    /// Returns the Player, if any.
    #[inline(always)]
    pub fn player(self) -> Option<Player> {
        match self {
            Piece::None => None,
            Piece::WhitePawn
            | Piece::WhiteKnight
            | Piece::WhiteBishop
            | Piece::WhiteRook
            | Piece::WhiteQueen
            | Piece::WhiteKing
            | Piece::WhiteEagle
            | Piece::WhiteHawk => Some(Player::White),
            _ => Some(Player::Black),
        }
    }

    /// Returns the Player (undefined for Piece::None).
    #[inline(always)]
    pub fn player_lossy(self) -> Player {
        if (self as u8) >= 9 {
            Player::Black
        } else {
            Player::White
        }
    }

    /// Returns (Player, PieceType).
    #[inline(always)]
    pub fn player_piece_lossy(self) -> (Player, PieceType) {
        (self.player_lossy(), self.type_of())
    }

    /// Returns the HFEN character for this piece.
    pub fn character(self) -> Option<char> {
        match self {
            Piece::None => None,
            _ => Some(self.character_lossy()),
        }
    }

    /// Returns the HFEN character (panics for None).
    pub fn character_lossy(self) -> char {
        match self {
            Piece::WhitePawn => 'P',
            Piece::BlackPawn => 'p',
            Piece::WhiteKnight => 'N',
            Piece::BlackKnight => 'n',
            Piece::WhiteBishop => 'B',
            Piece::BlackBishop => 'b',
            Piece::WhiteRook => 'R',
            Piece::BlackRook => 'r',
            Piece::WhiteQueen => 'Q',
            Piece::BlackQueen => 'q',
            Piece::WhiteKing => 'K',
            Piece::BlackKing => 'k',
            Piece::WhiteEagle => 'E',
            Piece::BlackEagle => 'e',
            Piece::WhiteHawk => 'H',
            Piece::BlackHawk => 'h',
            Piece::None => panic!("character_lossy called on Piece::None"),
        }
    }

    /// Parse from a HFEN character.
    pub fn from_char(c: char) -> Option<Piece> {
        match c {
            'P' => Some(Piece::WhitePawn),
            'p' => Some(Piece::BlackPawn),
            'N' => Some(Piece::WhiteKnight),
            'n' => Some(Piece::BlackKnight),
            'B' => Some(Piece::WhiteBishop),
            'b' => Some(Piece::BlackBishop),
            'R' => Some(Piece::WhiteRook),
            'r' => Some(Piece::BlackRook),
            'Q' => Some(Piece::WhiteQueen),
            'q' => Some(Piece::BlackQueen),
            'K' => Some(Piece::WhiteKing),
            'k' => Some(Piece::BlackKing),
            'E' => Some(Piece::WhiteEagle),
            'e' => Some(Piece::BlackEagle),
            'H' => Some(Piece::WhiteHawk),
            'h' => Some(Piece::BlackHawk),
            _ => None,
        }
    }

    /// Index for array access (0-16).
    #[inline(always)]
    pub fn index(self) -> usize {
        self as usize
    }
}

/// Flat mapping: index = player*8 + piece_type => Piece.
static ALL_PIECES_FLAT: [Piece; 18] = [
    Piece::None,        // White + None(0)
    Piece::WhitePawn,   // White + P(1)
    Piece::WhiteKnight, // White + N(2)
    Piece::WhiteBishop, // White + B(3)
    Piece::WhiteRook,   // White + R(4)
    Piece::WhiteQueen,  // White + Q(5)
    Piece::WhiteKing,   // White + K(6)
    Piece::WhiteEagle,  // White + E(7)
    Piece::WhiteHawk,   // White + H(8)
    Piece::BlackPawn,   // Black + P(1) -> idx 9
    Piece::BlackKnight, // Black + N(2)
    Piece::BlackBishop, // Black + B(3)
    Piece::BlackRook,   // Black + R(4)
    Piece::BlackQueen,  // Black + Q(5)
    Piece::BlackKing,   // Black + K(6)
    Piece::BlackEagle,  // Black + E(7)
    Piece::BlackHawk,   // Black + H(8)
    Piece::None,        // padding
];

impl fmt::Display for Piece {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.character() {
            Some(c) => write!(f, "{}", c),
            None => write!(f, "."),
        }
    }
}

impl fmt::Debug for Piece {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = match *self {
            Piece::None => "None",
            Piece::WhitePawn => "WhitePawn",
            Piece::WhiteKnight => "WhiteKnight",
            Piece::WhiteBishop => "WhiteBishop",
            Piece::WhiteRook => "WhiteRook",
            Piece::WhiteQueen => "WhiteQueen",
            Piece::WhiteKing => "WhiteKing",
            Piece::WhiteEagle => "WhiteEagle",
            Piece::WhiteHawk => "WhiteHawk",
            Piece::BlackPawn => "BlackPawn",
            Piece::BlackKnight => "BlackKnight",
            Piece::BlackBishop => "BlackBishop",
            Piece::BlackRook => "BlackRook",
            Piece::BlackQueen => "BlackQueen",
            Piece::BlackKing => "BlackKing",
            Piece::BlackEagle => "BlackEagle",
            Piece::BlackHawk => "BlackHawk",
        };
        write!(f, "{}", s)
    }
}
