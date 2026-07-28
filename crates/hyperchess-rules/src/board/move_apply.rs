// SPDX-License-Identifier: GPL-3.0-or-later
// HyperChess Core — hyperchess-rules
// File: crates/hyperchess-rules/src/board/move_apply.rs
// Version: 1.0.0
// Copyright (c) 2026 HyperChess Developer Team

//! Move application and reversal: `apply_move`, `undo_move`, and null moves.
//!
//! This is the most behavior-sensitive part of the board: it must keep the
//! piece bitboards, occupancy, piece-location map, king squares, castling
//! rights, en-passant square, Zobrist hash and check state perfectly in sync,
//! and `undo_move` must restore the previous state exactly (gated by
//! `test_zobrist_consistency` and `test_random_games_terminate`).

use super::board_state::BoardState;
use super::Board;
use crate::core::masks::*;
use crate::core::piece_identity::NO_PIECE_ID;
use crate::core::piece_move::HyperMove;
use crate::core::sq::{NO_SQ, SQ};
use crate::core::{Piece, PieceType, Player};
use crate::helper::zobrist::ZOBRIST;

impl Board {
    /// Applies a move to the board. Saves state for undo.
    pub fn apply_move(&mut self, m: HyperMove) {
        // Save the full current state including the prev chain (for multi-ply undo).
        let prev_state = BoardState {
            castling: self.state.castling,
            ep_square: self.state.ep_square,
            rule50: self.state.rule50,
            fullmove: self.state.fullmove,
            zobrist: self.state.zobrist,
            captured_piece: self.state.captured_piece,
            captured_identity: self.state.captured_identity,
            checkers: self.state.checkers,
            last_move: self.state.last_move,
            prev: self.state.prev.take(), // Preserve undo chain
        };
        self.state.prev = Some(Box::new(prev_state));
        self.state.last_move = m;
        self.history.push(self.state.zobrist);

        let us = self.turn;
        let them = !us;
        let src = m.get_src();
        let dst = m.get_dest();
        let piece = self.piece_locs.piece_at(src);
        let (_, piece_type) = piece.player_piece_lossy();

        // Update Zobrist for side to move
        self.state.zobrist ^= ZOBRIST.side;

        // Remove old EP from Zobrist
        if self.state.ep_square.is_okay() {
            self.state.zobrist ^= ZOBRIST.ep_file(self.state.ep_square.file_idx());
        }

        // Remove old castling from Zobrist
        self.state.zobrist ^= ZOBRIST.castle(self.state.castling.0);

        // Handle castling move
        if m.is_castle() {
            let side = if m.is_king_castle() { 0 } else { 1 };
            let rook_from = SQ(CASTLING_ROOK_START[us as usize][side]);
            let rook_to = SQ(CASTLING_ROOK_DST[us as usize][side]);
            let king_to = SQ(CASTLING_KING_DST[us as usize][side]);

            let rook_piece = Piece::make_lossy(us, PieceType::R);

            // Move king
            let king_identity = self.remove_piece_with_id(src, piece, us, PieceType::K);
            self.place_piece_with_id(king_to, piece, us, PieceType::K, king_identity);
            self.king_sq[us as usize] = king_to;

            // Move rook
            let rook_identity = self.remove_piece_with_id(rook_from, rook_piece, us, PieceType::R);
            self.place_piece_with_id(rook_to, rook_piece, us, PieceType::R, rook_identity);

            self.state.castling.remove_player(us);
            self.state.ep_square = NO_SQ;
            self.state.captured_piece = Piece::None;
            self.state.captured_identity = NO_PIECE_ID;
        } else if m.is_en_passant() {
            // En passant capture
            let captured_sq = SQ((dst.0 as i16 - us.pawn_push()) as u8);
            let captured = self.piece_locs.piece_at(captured_sq);
            self.state.captured_piece = captured;
            self.state.captured_identity =
                self.remove_piece_with_id(captured_sq, captured, them, PieceType::P);

            let moving_identity = self.remove_piece_with_id(src, piece, us, piece_type);
            self.place_piece_with_id(dst, piece, us, piece_type, moving_identity);

            self.state.ep_square = NO_SQ;
        } else {
            // Handle capture
            let captured = self.piece_locs.piece_at(dst);
            self.state.captured_piece = captured;
            if captured != Piece::None {
                let (cap_player, cap_type) = captured.player_piece_lossy();
                self.state.captured_identity =
                    self.remove_piece_with_id(dst, captured, cap_player, cap_type);
                self.state.rule50 = 0;
            } else {
                self.state.captured_identity = NO_PIECE_ID;
            }

            // Move piece
            let moving_identity = self.remove_piece_with_id(src, piece, us, piece_type);

            if m.is_promo() {
                let promo_pt = m.promo_piece();
                let promo_piece = Piece::make_lossy(us, promo_pt);
                self.place_piece_with_id(dst, promo_piece, us, promo_pt, moving_identity);
            } else {
                self.place_piece_with_id(dst, piece, us, piece_type, moving_identity);
                if piece_type == PieceType::K {
                    self.king_sq[us as usize] = dst;
                }
            }

            // En passant square for double pawn push
            if m.is_double_push() {
                let ep_sq = SQ(((src.0 as i16 + dst.0 as i16) / 2) as u8);
                self.state.ep_square = ep_sq;
                self.state.zobrist ^= ZOBRIST.ep_file(ep_sq.file_idx());
            } else {
                self.state.ep_square = NO_SQ;
            }

            // Update castling rights
            self.state.castling.update_for_move(src, dst);
        }

        // Update Zobrist for new castling
        self.state.zobrist ^= ZOBRIST.castle(self.state.castling.0);

        // Update rule50 and fullmove
        if piece_type == PieceType::P || self.state.captured_piece != Piece::None {
            self.state.rule50 = 0;
        } else {
            self.state.rule50 += 1;
        }
        if us == Player::Black {
            self.state.fullmove += 1;
        }

        self.turn = them;
        self.ply += 1;

        // Update checkers
        self.state.checkers = self.compute_checkers();
    }

    /// Undoes the last move.
    pub fn undo_move(&mut self) {
        let prev = match self.state.prev.take() {
            Some(p) => p,
            None => panic!("undo_move called with no previous state"),
        };
        self.history.pop();

        let m = self.state.last_move;
        self.ply -= 1;
        self.turn = !self.turn;
        let us = self.turn;
        let them = !us;
        let src = m.get_src();
        let dst = m.get_dest();

        if m.is_castle() {
            let side = if m.is_king_castle() { 0 } else { 1 };
            let rook_from = SQ(CASTLING_ROOK_START[us as usize][side]);
            let rook_to = SQ(CASTLING_ROOK_DST[us as usize][side]);
            let king_to = SQ(CASTLING_KING_DST[us as usize][side]);
            let king_piece = self.piece_locs.piece_at(king_to);
            let rook_piece = self.piece_locs.piece_at(rook_to);

            // Undo king move
            let king_identity = self.remove_piece_with_id(king_to, king_piece, us, PieceType::K);
            self.place_piece_with_id(src, king_piece, us, PieceType::K, king_identity);
            self.king_sq[us as usize] = src;

            // Undo rook move
            let rook_identity = self.remove_piece_with_id(rook_to, rook_piece, us, PieceType::R);
            self.place_piece_with_id(rook_from, rook_piece, us, PieceType::R, rook_identity);
        } else if m.is_en_passant() {
            let piece = self.piece_locs.piece_at(dst);
            let moving_identity = self.remove_piece_with_id(dst, piece, us, PieceType::P);
            self.place_piece_with_id(src, piece, us, PieceType::P, moving_identity);

            // Restore captured pawn
            let captured_sq = SQ((dst.0 as i16 - us.pawn_push()) as u8);
            self.place_piece_with_id(
                captured_sq,
                self.state.captured_piece,
                them,
                PieceType::P,
                self.state.captured_identity,
            );
        } else {
            let _moved_piece_type;
            if m.is_promo() {
                let promo_pt = m.promo_piece();
                let promo_piece = Piece::make_lossy(us, promo_pt);
                let moving_identity = self.remove_piece_with_id(dst, promo_piece, us, promo_pt);
                let pawn = Piece::make_lossy(us, PieceType::P);
                self.place_piece_with_id(src, pawn, us, PieceType::P, moving_identity);
                _moved_piece_type = PieceType::P;
            } else {
                let piece = self.piece_locs.piece_at(dst);
                let pt = piece.type_of();
                let moving_identity = self.remove_piece_with_id(dst, piece, us, pt);
                self.place_piece_with_id(src, piece, us, pt, moving_identity);
                _moved_piece_type = pt;
                if pt == PieceType::K {
                    self.king_sq[us as usize] = src;
                }
            }

            // Restore captured piece
            if self.state.captured_piece != Piece::None {
                let cap = self.state.captured_piece;
                let (cap_player, cap_type) = cap.player_piece_lossy();
                self.place_piece_with_id(
                    dst,
                    cap,
                    cap_player,
                    cap_type,
                    self.state.captured_identity,
                );
            }
        }

        self.state = *prev;
    }

    /// Applies a null move (pass turn, used in search).
    pub fn apply_null_move(&mut self) {
        let prev_state = BoardState {
            castling: self.state.castling,
            ep_square: self.state.ep_square,
            rule50: self.state.rule50,
            fullmove: self.state.fullmove,
            zobrist: self.state.zobrist,
            captured_piece: self.state.captured_piece,
            captured_identity: self.state.captured_identity,
            checkers: self.state.checkers,
            last_move: self.state.last_move,
            prev: self.state.prev.take(),
        };
        self.state.prev = Some(Box::new(prev_state));
        self.state.last_move = HyperMove::null();

        self.state.zobrist ^= ZOBRIST.side;
        if self.state.ep_square.is_okay() {
            self.state.zobrist ^= ZOBRIST.ep_file(self.state.ep_square.file_idx());
            self.state.ep_square = NO_SQ;
        }
        self.state.captured_piece = Piece::None;
        self.state.captured_identity = NO_PIECE_ID;
        self.state.rule50 += 1;

        self.turn = !self.turn;
        self.ply += 1;
        self.state.checkers = self.compute_checkers();
    }

    /// Undoes a null move.
    pub fn undo_null_move(&mut self) {
        let prev = self.state.prev.take().expect("undo_null_move with no prev");
        self.ply -= 1;
        self.turn = !self.turn;
        self.state = *prev;
    }
}
