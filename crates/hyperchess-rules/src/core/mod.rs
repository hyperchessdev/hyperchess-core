// SPDX-License-Identifier: GPL-3.0-or-later
// HyperChess Core — hyperchess-rules
// File: crates/hyperchess-rules/src/core/mod.rs
// Version: 1.0.0
// Copyright (c) 2026 HyperChess Developer Team

//! Core types for HyperChess: Player, PieceType, Piece, File, Rank, GenTypes.
//!
//! The individual types live in focused submodules and are re-exported here so
//! the rest of the crate can keep importing them as `crate::core::{...}`.

pub mod bit_twiddles;
pub mod bitboard;
pub mod masks;
pub mod move_list;
pub mod piece_move;
pub mod score;
pub mod sq;

pub mod castling;
pub mod file_rank;
pub mod piece;
pub mod piece_identity;
pub mod piece_type;
pub mod player;

pub use castling::{CastleType, Phase};
pub use file_rank::{File, Rank, ALL_FILES, ALL_RANKS};
pub use piece::{Piece, PIECE_ENUM_CNT};
pub use piece_type::{GenTypes, PieceType, ALL_PIECE_TYPES};
pub use player::{Player, ALL_PLAYERS};
