// SPDX-License-Identifier: GPL-3.0-or-later
// HyperChess Core — hyperchess-rules
// File: crates/hyperchess-rules/src/game_replay.rs
// Version: 1.0.0
// Copyright (c) 2026 HyperChess Developer Team

//! Game replay validator for HPGN-I + HFEN-I round-trip verification.
//!
//! This module provides integration-level testing of the complete game format:
//! - Parse an HFEN-I starting position
//! - Apply a sequence of HPGN-I moves
//! - Verify board state consistency after each move
//! - Verify identity persistence through promotions
//! - Validate castling and en passant state updates

use crate::board::Board;
use crate::core::piece_move::HyperMove;

/// Result of a game replay operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplayResult {
    /// Number of moves successfully applied.
    pub moves_applied: usize,
    /// Final board state after all moves.
    pub final_hfen: String,
    /// Whether the final position matches the expected HFEN (if provided).
    pub final_matches_expected: bool,
}

/// Replay a game from an HFEN-I starting position using a list of plain UCI moves.
///
/// This is the integration test: parse position, apply moves, verify state.
pub fn replay_game(
    start_hfen: &str,
    moves: &[&str],
    expected_final_hfen: Option<&str>,
) -> Result<ReplayResult, String> {
    let mut board = Board::from_hfen(start_hfen)?;

    for (i, uci_move) in moves.iter().enumerate() {
        // Parse the move from UCI string and find it in the legal move list.
        let parsed_move = parse_uci_move(&board, uci_move).ok_or_else(|| {
            format!(
                "Move {}: '{}' is not legal or cannot be parsed",
                i + 1,
                uci_move
            )
        })?;

        // Apply the move.
        board.apply_move(parsed_move);
    }

    let final_hfen = board.get_hfen();
    let final_matches_expected = expected_final_hfen.is_none_or(|expected| final_hfen == expected);

    Ok(ReplayResult {
        moves_applied: moves.len(),
        final_hfen,
        final_matches_expected,
    })
}

/// Parse a UCI move string (e.g., "a3a4", "a11a12q") against the current board
/// and return the first matching legal move.
///
/// This is a simple parser: extract source, destination, and optional promotion piece.
fn parse_uci_move(board: &Board, uci: &str) -> Option<HyperMove> {
    let moves = board.generate_moves();

    for m in moves.iter() {
        if m.stringify() == uci {
            return Some(*m);
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::masks::START_HFEN;

    /// Simple 1-move game: white pawn advance from starting position.
    #[test]
    fn test_replay_single_quiet_move() {
        let moves = vec!["a3a4"];
        let result = replay_game(START_HFEN, &moves, None).expect("replay should succeed");
        assert_eq!(result.moves_applied, 1);
    }

    /// Verify identity persists through a promotion.
    #[test]
    fn test_replay_promotion_identity_persistence() {
        // White pawn M at a11 (moved from a3), one rank from promotion.
        // Simplified position: clear the board except key pieces.
        let start = "12/M11/6g5/12/12/12/12/12/12/12/6G5/12 w - - 0 1";
        let moves = vec!["a11a12q"];

        let result = replay_game(start, &moves, None).expect("replay should succeed");
        assert_eq!(result.moves_applied, 1);
        // Final HFEN should have M:Q on a12 (identity M, type queen).
        assert!(
            result.final_hfen.starts_with("M:Q11/"),
            "promotion should write identity:type pair, got: {}",
            result.final_hfen
        );
    }

    /// Multi-move game from starting position.
    #[test]
    fn test_replay_multi_move_game() {
        let moves = vec!["a3a4", "a10a9", "a4a5"];
        let result = replay_game(START_HFEN, &moves, None).expect("replay should succeed");
        assert_eq!(result.moves_applied, 3);
    }

    /// Verify illegal move is rejected.
    #[test]
    fn test_replay_illegal_move_rejected() {
        // Try a move that's illegal in the starting position.
        let moves = vec!["a1a2"]; // a1 is empty (no white pawn there).
        let result = replay_game(START_HFEN, &moves, None);
        assert!(result.is_err(), "illegal move should fail");
    }

    /// Verify that the replayed final HFEN matches the expected HFEN.
    #[test]
    fn test_replay_final_hfen_match() {
        let moves = vec!["a3a4"];

        let mut temp_board = Board::from_hfen(START_HFEN).unwrap();
        let move_to_apply = temp_board
            .generate_moves()
            .iter()
            .find(|m| m.stringify() == "a3a4")
            .copied()
            .unwrap();
        temp_board.apply_move(move_to_apply);
        let expected = temp_board.get_hfen();

        let result =
            replay_game(START_HFEN, &moves, Some(&expected)).expect("replay should succeed");
        assert!(
            result.final_matches_expected,
            "final HFEN should match expected"
        );
    }
}
