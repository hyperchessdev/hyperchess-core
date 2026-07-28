// SPDX-License-Identifier: GPL-3.0-or-later
// HyperChess Core — hyperchess-rules
// File: crates/hyperchess-rules/src/lib.rs
// Version: 1.0.0
// Copyright (c) 2026 HyperChess Developer Team

//! HyperChess rules — a 12x12 chess variant with Eagle and Hawk pieces.
//!
//! Board representation, move generation, and legality only. Search
//! (alpha-beta/MCTS/etc.) lives in the separate `hyperchess-search` crate,
//! which depends on this one — not the other way around.

#![allow(clippy::cast_lossless)]
#![allow(clippy::unusual_byte_groupings)]
#![allow(dead_code)]
// Both inherited as-is from the source repo, not introduced by this
// extraction. Deliberately left as explicit allows rather than hand-rewritten
// during Phase 1 ("copy + path fix, no logic changes" —
// docs/hyperchess-core-extraction-plan.md §12):
// - needless_range_loop: ~10 sites in Zobrist/PSQT/distance-table code
//   (helper/{boards,psqt,zobrist}.rs, board/hfen.rs). clippy's own --fix
//   deliberately does NOT auto-apply this lint (the correct iterator rewrite
//   depends on loop-body semantics), and hand-transcribing index arithmetic
//   in hot, correctness-sensitive lookup-table code isn't a Phase-1-scope
//   risk worth taking for a style lint.
// - too_many_arguments: 1 site (board/movegen/king.rs::can_castle, 8/7) — no
//   mechanical fix exists at all; fixing it means an actual signature/API
//   change (e.g. grouping args into a struct) touching every call site, not
//   a copy-and-relocate change.
#![allow(clippy::needless_range_loop)]
#![allow(clippy::too_many_arguments)]

pub mod board;
pub mod core;
pub mod game_replay;
pub mod helper;
pub mod io;
pub mod notation;
pub mod tools;

pub use board::Board;
pub use core::bitboard::BitBoard;
pub use core::move_list::{MoveList, ScoringMoveList};
pub use core::piece_move::{HyperMove, ScoringMove};
pub use core::sq::SQ;
pub use core::{File, GenTypes, Piece, PieceType, Player, Rank};
pub use helper::Helper;
pub use notation::GameRecord;
