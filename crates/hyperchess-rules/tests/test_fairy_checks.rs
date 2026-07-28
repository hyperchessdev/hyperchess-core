// SPDX-License-Identifier: GPL-3.0-or-later
// HyperChess Core — hyperchess-rules
// File: crates/hyperchess-rules/tests/test_fairy_checks.rs
// Version: 1.0.0
// Copyright (c) 2026 HyperChess Developer Team

//! Fairy check verification tests. (Phase 02)

use hyperchess_rules::core::sq::SQ;
use hyperchess_rules::core::Player;
use hyperchess_rules::Board;

#[test]
fn test_hawk_check_blocker_interposition() {
    // White King at c5 (SQ(50)), Black Hawk at a3 (SQ(24)).
    // There is a blocker between them: say, White pawn on b4 (SQ(37)).
    // Since Hawk check is unblockable, the presence of White pawn on b4 does NOT block the check.
    // The King must move or the Hawk must be captured. No blocker move can resolve the check.
    let hfen = "12/12/12/12/12/12/12/2K9/1P10/h11/12/11k w - - 0 1";
    let board = Board::from_hfen(hfen).unwrap();

    assert!(board.in_check());

    let moves = board.generate_moves();
    for m in moves.iter() {
        let src = m.get_src();
        let dest = m.get_dest();
        if src != board.king_sq(Player::White) {
            // Non-king moves must be captures of the checking Hawk at a3
            assert_eq!(dest, SQ::make(0, 2), "Non-king move must capture checker");
        }
    }
}

#[test]
fn test_eagle_check_blocker_interposition() {
    // White King at c5 (SQ(50)), Black Eagle at c1 (SQ(2)).
    // Blocker at c3 (SQ(26)).
    // Eagle check is orthogonal and unblockable. The blocker does not prevent check.
    let hfen = "12/12/12/12/12/12/12/2K9/2P9/12/2e9/11k w - - 0 1";
    let board = Board::from_hfen(hfen).unwrap();

    assert!(board.in_check());

    let moves = board.generate_moves();
    for m in moves.iter() {
        let src = m.get_src();
        let dest = m.get_dest();
        if src != board.king_sq(Player::White) {
            assert_eq!(
                dest,
                SQ::make(2, 0),
                "Non-king move must capture checking Eagle at c1"
            );
        }
    }
}

#[test]
fn test_double_check_with_fairy() {
    // White King at c5, Black Hawk at a3 (gives check), Black Bishop at e7 (gives check).
    // This is a double check. Only King moves should be legal.
    let hfen = "12/12/12/12/12/4b7/12/2K9/12/h11/12/11k w - - 0 1";
    let board = Board::from_hfen(hfen).unwrap();

    assert!(board.in_check());

    let moves = board.generate_moves();
    for m in moves.iter() {
        let src = m.get_src();
        assert_eq!(
            src,
            board.king_sq(Player::White),
            "In double check, only King moves are legal"
        );
    }
}

#[test]
fn test_checkmate_with_fairy_delivering_check() {
    // King is trapped, Hawk checks, no piece can capture it, no king move is legal.
    // White King on a1, surrounded by: White pawn on a2, White Knight on b2, White pawn on b1.
    // Black Hawk on c3 (attacks a1).
    // Checker is at c3. King has no moves, no capture is possible. Checkmate!
    let hfen = "12/12/12/12/12/12/12/12/12/2h9/PN10/KP9k w - - 0 1";
    let board = Board::from_hfen(hfen).unwrap();

    assert!(board.in_check());
    assert!(board.is_game_over());
    assert_eq!(board.game_result(), 2, "Black wins by checkmate");
}
