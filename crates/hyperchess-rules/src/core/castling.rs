// SPDX-License-Identifier: GPL-3.0-or-later
// HyperChess Core — hyperchess-rules
// File: crates/hyperchess-rules/src/core/castling.rs
// Version: 1.0.0
// Copyright (c) 2026 HyperChess Developer Team

//! Castling side and game-phase enums.

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
#[repr(u8)]
/// Which side of the board a castle happens on. Discriminants double as the
/// `side` index into the `CASTLING_*` tables in [`crate::core::masks`].
pub enum CastleType {
    /// Toward the h/i/j files.
    KingSide = 0,
    /// Toward the c/e/f files.
    QueenSide = 1,
}

/// Game phases.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum Phase {
    /// Middlegame — selects the `mg` half of a [`crate::core::score::Score`].
    MG = 0,
    /// Endgame — selects the `eg` half.
    EG = 1,
}
