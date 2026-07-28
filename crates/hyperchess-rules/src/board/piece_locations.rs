// SPDX-License-Identifier: GPL-3.0-or-later
// HyperChess Core — hyperchess-rules
// File: crates/hyperchess-rules/src/board/piece_locations.rs
// Version: 1.0.0
// Copyright (c) 2026 HyperChess Developer Team

//! Piece locations — a [Piece; 144] array mapping each square to its piece.

use crate::core::masks::SQ_CNT;
use crate::core::sq::SQ;
use crate::core::{Piece, PieceType};

/// Maps each square to the piece on it (Piece::None if empty).
#[derive(Clone)]
pub struct PieceLocations {
    data: [Piece; SQ_CNT],
}

impl PieceLocations {
    /// Creates an empty board.
    pub fn new() -> Self {
        PieceLocations {
            data: [Piece::None; SQ_CNT],
        }
    }

    /// Places a piece on a square.
    #[inline(always)]
    pub fn place(&mut self, sq: SQ, piece: Piece) {
        self.data[sq.0 as usize] = piece;
    }

    /// Removes a piece from a square, returning what was there.
    #[inline(always)]
    pub fn remove(&mut self, sq: SQ) -> Piece {
        let piece = self.data[sq.0 as usize];
        self.data[sq.0 as usize] = Piece::None;
        piece
    }

    /// Returns the piece on a square.
    #[inline(always)]
    pub fn piece_at(&self, sq: SQ) -> Piece {
        self.data[sq.0 as usize]
    }

    /// Returns the piece type on a square.
    #[inline(always)]
    pub fn piece_type_at(&self, sq: SQ) -> PieceType {
        self.data[sq.0 as usize].type_of()
    }

    /// Moves a piece from one square to another.
    #[inline]
    pub fn move_piece(&mut self, from: SQ, to: SQ) {
        let piece = self.data[from.0 as usize];
        self.data[to.0 as usize] = piece;
        self.data[from.0 as usize] = Piece::None;
    }

    /// Returns a reference to the raw array.
    pub fn as_slice(&self) -> &[Piece; SQ_CNT] {
        &self.data
    }
}

impl Default for PieceLocations {
    fn default() -> Self {
        PieceLocations::new()
    }
}
