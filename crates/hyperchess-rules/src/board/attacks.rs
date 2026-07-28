// SPDX-License-Identifier: GPL-3.0-or-later
// HyperChess Core — hyperchess-rules
// File: crates/hyperchess-rules/src/board/attacks.rs
// Version: 1.0.0
// Copyright (c) 2026 HyperChess Developer Team

//! Attack and check queries: which pieces attack a square, and check detection.

use super::Board;
use crate::core::bitboard::BitBoard;
use crate::core::sq::SQ;
use crate::core::{PieceType, Player};
use crate::helper::prelude::*;

impl Board {
    /// Returns a bitboard of all pieces attacking a given square.
    pub fn attackers_to(&self, sq: SQ, occupied: BitBoard) -> BitBoard {
        let mut attackers = BitBoard::EMPTY;

        // Pawns
        attackers |=
            pawn_attacks_sq(Player::Black, sq) & self.piece_bb(Player::White, PieceType::P);
        attackers |=
            pawn_attacks_sq(Player::White, sq) & self.piece_bb(Player::Black, PieceType::P);

        // Knights
        let n_attacks = knight_attacks(sq);
        attackers |= n_attacks
            & (self.piece_bb(Player::White, PieceType::N)
                | self.piece_bb(Player::Black, PieceType::N));

        // Kings
        let k_attacks = king_attacks(sq);
        attackers |= k_attacks
            & (self.piece_bb(Player::White, PieceType::K)
                | self.piece_bb(Player::Black, PieceType::K));

        // Rooks and Queens (sliding orthogonal)
        let r_attacks = rook_attacks(sq, occupied);
        let rq = self.piece_bb(Player::White, PieceType::R)
            | self.piece_bb(Player::Black, PieceType::R)
            | self.piece_bb(Player::White, PieceType::Q)
            | self.piece_bb(Player::Black, PieceType::Q);
        attackers |= r_attacks & rq;

        // Bishops and Queens (sliding diagonal)
        let b_attacks = bishop_attacks(sq, occupied);
        let bq = self.piece_bb(Player::White, PieceType::B)
            | self.piece_bb(Player::Black, PieceType::B)
            | self.piece_bb(Player::White, PieceType::Q)
            | self.piece_bb(Player::Black, PieceType::Q);
        attackers |= b_attacks & bq;

        // Eagles (jumping orthogonal)
        let e_attacks = eagle_attacks(sq, occupied);
        attackers |= e_attacks
            & (self.piece_bb(Player::White, PieceType::E)
                | self.piece_bb(Player::Black, PieceType::E));

        // Hawks (jumping diagonal)
        let h_attacks = hawk_attacks(sq, occupied);
        attackers |= h_attacks
            & (self.piece_bb(Player::White, PieceType::H)
                | self.piece_bb(Player::Black, PieceType::H));

        attackers
    }

    /// Returns attackers of a specific player to a square.
    pub fn attackers_to_player(&self, sq: SQ, player: Player, occupied: BitBoard) -> BitBoard {
        let mut attackers = BitBoard::EMPTY;

        attackers |= pawn_attacks_sq(!player, sq) & self.piece_bb(player, PieceType::P);
        attackers |= knight_attacks(sq) & self.piece_bb(player, PieceType::N);
        attackers |= king_attacks(sq) & self.piece_bb(player, PieceType::K);
        attackers |= rook_attacks(sq, occupied)
            & (self.piece_bb(player, PieceType::R) | self.piece_bb(player, PieceType::Q));
        attackers |= bishop_attacks(sq, occupied)
            & (self.piece_bb(player, PieceType::B) | self.piece_bb(player, PieceType::Q));
        attackers |= eagle_attacks(sq, occupied) & self.piece_bb(player, PieceType::E);
        attackers |= hawk_attacks(sq, occupied) & self.piece_bb(player, PieceType::H);

        attackers
    }

    /// Computes the bitboard of pieces giving check to the side to move.
    pub(crate) fn compute_checkers(&self) -> BitBoard {
        let us = self.turn();
        let them = !us;
        let king = self.king_sq(us);
        self.attackers_to_player(king, them, self.occupied())
    }

    /// Returns true if the given player's king is attacked.
    pub fn is_attacked(&self, player: Player) -> bool {
        let king = self.king_sq(player);
        let them = !player;
        self.attackers_to_player(king, them, self.occupied())
            .is_not_empty()
    }
}
