// SPDX-License-Identifier: GPL-3.0-or-later
// HyperChess Core — hyperchess-rules
// File: crates/hyperchess-rules/src/helper/zobrist.rs
// Version: 1.0.0
// Copyright (c) 2026 HyperChess Developer Team

//! Zobrist hashing for HyperChess (144 squares, 10 piece types, 2 players).

use crate::core::masks::*;
use crate::core::sq::SQ;
use crate::core::Piece;
use crate::tools::prng::PRNG;

/// Number of unique pieces for Zobrist (player * piece_type = 2 * 8 real pieces = 16 + padding).
const ZOBRIST_PIECE_CNT: usize = 18; // indexed by Piece enum value (0..16+1)

/// Zobrist hash keys.
pub struct ZobristKeys {
    /// piece_sq[piece][square]
    pub piece_sq: [[u64; SQ_CNT]; ZOBRIST_PIECE_CNT],
    /// en_passant[file]
    pub en_passant: [u64; FILE_CNT],
    /// castling[16 combinations]
    pub castling: [u64; ALL_CASTLING_RIGHTS],
    /// Side to move.
    pub side: u64,
    /// Rule version salt.
    pub version_salt: u64,
}

/// Lazily-initialized global key set. `std::sync::LazyLock` (not an external
/// macro crate) — dereferences exactly like the old `lazy_static` binding, so
/// call sites read `ZOBRIST.side`, `ZOBRIST.castle(..)`, etc. unchanged.
///
/// Keys come from a fixed PRNG seed rather than entropy, so hashes are
/// reproducible across runs — a persisted transposition table or a logged hash
/// from one run stays meaningful in the next.
pub static ZOBRIST: std::sync::LazyLock<ZobristKeys> = std::sync::LazyLock::new(ZobristKeys::init);

impl ZobristKeys {
    /// Fill every key from the seeded PRNG. See [`ZOBRIST`] for why the seed is
    /// fixed rather than drawn from entropy.
    fn init() -> Self {
        let mut prng = PRNG::init(1070372);

        let mut piece_sq = [[0u64; SQ_CNT]; ZOBRIST_PIECE_CNT];
        for piece_idx in 0..ZOBRIST_PIECE_CNT {
            for sq in 0..SQ_CNT {
                piece_sq[piece_idx][sq] = prng.rand();
            }
        }

        let mut en_passant = [0u64; FILE_CNT];
        for f in 0..FILE_CNT {
            en_passant[f] = prng.rand();
        }

        let mut castling = [0u64; ALL_CASTLING_RIGHTS];
        for c in 0..ALL_CASTLING_RIGHTS {
            castling[c] = prng.rand();
        }

        let side = prng.rand();

        ZobristKeys {
            piece_sq,
            en_passant,
            castling,
            side,
            version_salt: 0xb1cb_fed6_6a8c_6417,
        }
    }

    /// Returns the Zobrist key for a piece on a square.
    #[inline(always)]
    pub fn piece_at(&self, piece: Piece, sq: SQ) -> u64 {
        self.piece_sq[piece as usize][sq.0 as usize]
    }

    /// Returns the Zobrist key for an en passant file.
    #[inline(always)]
    pub fn ep_file(&self, file: u8) -> u64 {
        self.en_passant[file as usize]
    }

    /// Returns the Zobrist key for castling rights.
    #[inline(always)]
    pub fn castle(&self, rights: u8) -> u64 {
        self.castling[rights as usize]
    }
}
