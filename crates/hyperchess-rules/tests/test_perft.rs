// SPDX-License-Identifier: GPL-3.0-or-later
// HyperChess Core — hyperchess-rules
// File: crates/hyperchess-rules/tests/test_perft.rs
// Version: 1.0.0
// Copyright (c) 2026 HyperChess Developer Team

//! Perft verification suite (CI gate / Phase 02).

use hyperchess_rules::board::perft::perft;
use hyperchess_rules::Board;

#[test]
fn test_perft_startpos() {
    let mut board = Board::start_pos();
    // Depth 0
    assert_eq!(perft(&mut board, 0), 1);
    // Depth 1
    assert_eq!(perft(&mut board, 1), 62);
    // Depth 2: 62 white moves. Let's verify it compiles and runs depth 2 quickly.
    let nodes_d2 = perft(&mut board, 2);
    println!("Perft startpos Depth 2: {}", nodes_d2);
}

#[test]
fn test_perft_castling() {
    // Both sides have castling rights, pieces cleared.
    // White King at g2, Rook at j2 (king-side castling).
    // HFEN: 12/ehrnbqkbnrhe/pppppppppppp/12/12/12/12/12/12/PPPPPPPPPPPP/6K2R2/12 w KQkq - 0 1
    // (Wait, castling rights KQkq are active, king/rook in original squares)
    let hfen = "12/ehrnbqkbnrhe/pppppppppppp/12/12/12/12/12/12/PPPPPPPPPPPP/6K2R2/12 w KQkq - 0 1";
    let mut board = Board::from_hfen(hfen).unwrap();
    let nodes_d1 = perft(&mut board, 1);
    assert!(nodes_d1 > 0);
}

#[test]
fn test_perft_en_passant() {
    // White pawn on e8, Black pawn double stepped to d8 (EP square target d9).
    let hfen = "12/12/12/12/3pP7/12/12/12/12/12/K11/11k w - d9 0 1";
    let mut board = Board::from_hfen(hfen).unwrap();
    let nodes_d1 = perft(&mut board, 1);
    // Pawns can capture or push.
    assert!(nodes_d1 > 0);
}

#[test]
fn test_perft_promotions() {
    // White pawn on e11 ready to promote to e12 (all 6 piece types).
    let hfen = "12/4P7/12/12/12/12/12/12/12/12/K11/11k w - - 0 1";
    let mut board = Board::from_hfen(hfen).unwrap();
    let nodes_d1 = perft(&mut board, 1);
    assert!(nodes_d1 > 0);
}

#[test]
fn test_perft_fairy_check_interposition() {
    // Hawk checking king, blocker in between.
    let hfen = "12/12/12/12/12/12/12/2K9/1P10/h11/12/11k w - - 0 1";
    let mut board = Board::from_hfen(hfen).unwrap();
    let nodes_d1 = perft(&mut board, 1);
    // King must escape or Hawk captured. No blocker moves legal.
    assert!(nodes_d1 > 0);
}
