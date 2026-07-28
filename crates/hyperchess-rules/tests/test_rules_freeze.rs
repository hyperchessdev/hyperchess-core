// SPDX-License-Identifier: GPL-3.0-or-later
// HyperChess Core — hyperchess-rules
// File: crates/hyperchess-rules/tests/test_rules_freeze.rs
// Version: 1.0.0
// Copyright (c) 2026 HyperChess Developer Team

//! Rules freezing verification tests (Phase 00).

use hyperchess_rules::core::masks::START_HFEN;
use hyperchess_rules::core::sq::SQ;
use hyperchess_rules::core::{PieceType, Player};
use hyperchess_rules::Board;

#[test]
fn test_start_pos_fen() {
    let board = Board::start_pos();
    assert_eq!(board.get_hfen(), START_HFEN);
}

#[test]
fn test_all_piece_types_present() {
    let board = Board::start_pos();
    // Verify presence of all piece types on both sides in the starting position
    let expected_white = [
        PieceType::P,
        PieceType::N,
        PieceType::B,
        PieceType::R,
        PieceType::Q,
        PieceType::K,
        PieceType::E,
        PieceType::H,
    ];
    for &pt in &expected_white {
        assert!(
            board.piece_bb(Player::White, pt).is_not_empty(),
            "White missing {:?}",
            pt
        );
        assert!(
            board.piece_bb(Player::Black, pt).is_not_empty(),
            "Black missing {:?}",
            pt
        );
    }
}

#[test]
fn test_pawn_double_step() {
    let mut board = Board::start_pos();
    let moves = board.generate_moves();

    // Check that white pawns can double step from rank 3 to rank 5
    // White pawns start on Rank 3 (index 2). Let's pick e3 (4, 2) which goes to e5 (4, 4)
    let e3 = SQ::make(4, 2);
    let e5 = SQ::make(4, 4);
    let double_push = moves
        .iter()
        .find(|m| m.get_src() == e3 && m.get_dest() == e5);
    assert!(
        double_push.is_some(),
        "Pawn double push from e3 to e5 not found"
    );

    // Apply double push
    board.apply_move(*double_push.unwrap());
    assert_eq!(
        board.state.ep_square,
        SQ::make(4, 3),
        "En passant target square must be e4"
    );
}

#[test]
fn test_en_passant_capture() {
    // White pawn on e5 (file 4, rank_idx 4), Black pawn on d5 (file 3, rank_idx 4).
    // Black just double-stepped d7->d5 (file 3, rank_idx 6->4). En passant target = d6 (file 3, rank_idx 5).
    // HFEN notation: 8th rank from top is index 7. Let's make:
    // White pawn at e8 (file 4, rank 7/index 7), Black double steps d10->d8 (file 3, rank index 9->7).
    let hfen = "12/12/12/12/3pP7/12/12/12/12/12/K11/11k w - d9 0 1";
    let mut board = Board::from_hfen(hfen).unwrap();
    let moves = board.generate_moves();

    let ep_move = moves.iter().find(|m| m.is_en_passant());
    assert!(ep_move.is_some(), "En passant move not found");

    let ep = ep_move.unwrap();
    assert_eq!(ep.get_src(), SQ::make(4, 7));
    assert_eq!(ep.get_dest(), SQ::make(3, 8)); // d9

    board.apply_move(*ep);

    // Captured Black pawn at d8 (index 3,7) should be removed
    assert_eq!(
        board.piece_at(SQ::make(3, 7)),
        hyperchess_rules::core::Piece::None
    );
    // Capturing White pawn is at d9 (index 3,8)
    assert_eq!(board.piece_at(SQ::make(3, 8)).type_of(), PieceType::P);
}

#[test]
fn test_all_promotions() {
    // White pawn on e11 (rank_idx 10) pushes to e12 (rank_idx 11)
    let hfen = "12/4P7/12/12/12/12/12/12/12/12/K11/11k w - - 0 1";
    let board = Board::from_hfen(hfen).unwrap();
    let moves = board.generate_moves();

    // Filter moves for pawn on e11
    let pawn_sq = SQ::make(4, 10);
    let promo_moves: Vec<_> = moves.iter().filter(|m| m.get_src() == pawn_sq).collect();

    // Must generate promotions to Q, R, B, N, E, H
    assert_eq!(promo_moves.len(), 6);

    let mut promo_types = Vec::new();
    for m in &promo_moves {
        assert!(m.is_promo());
        promo_types.push(m.promo_piece());
    }

    assert!(promo_types.contains(&PieceType::Q));
    assert!(promo_types.contains(&PieceType::R));
    assert!(promo_types.contains(&PieceType::B));
    assert!(promo_types.contains(&PieceType::N));
    assert!(promo_types.contains(&PieceType::E));
    assert!(promo_types.contains(&PieceType::H));
}

#[test]
fn test_castling_king_rook_squares() {
    // King g2 -> i2 (White kside)
    let hfen = "12/12/12/12/12/12/12/12/12/12/6K2R2/11k w K - 0 1";
    let mut board = Board::from_hfen(hfen).unwrap();
    let moves = board.generate_moves();
    let castle_kside = moves.iter().find(|m| m.is_king_castle());
    assert!(castle_kside.is_some());

    board.apply_move(*castle_kside.unwrap());
    assert_eq!(
        board.king_sq(Player::White),
        SQ::make(8, 1),
        "King must land on i2"
    );
    assert_eq!(
        board.piece_at(SQ::make(7, 1)).type_of(),
        PieceType::R,
        "Rook must land on h2"
    );
}

#[test]
fn test_stalemate() {
    // Stalemate: White has no moves but is not in check
    let hfen = "1r9k/12/12/12/12/12/12/12/12/p11/P11/K11 w - - 0 1";
    let board = Board::from_hfen(hfen).unwrap();
    assert!(board.generate_moves().is_empty());
    assert!(!board.in_check());
    assert_eq!(
        board.game_result(),
        3,
        "Stalemate must result in a draw (3)"
    );
}

#[test]
fn test_draw_112_moves() {
    let mut board = Board::start_pos();
    // Artificially inflate the 50-move rule counter (rule50) to 224 half-moves
    board.state.rule50 = 224;
    assert!(board.is_game_over());
    assert_eq!(
        board.game_result(),
        3,
        "112 full moves without pawn move/capture is a draw"
    );
}

#[test]
fn test_threefold_repetition() {
    let mut board = Board::start_pos();

    // We simulate 3 returns to the start position
    // Since history stores zobrist hashes:
    let hash = board.state.zobrist;
    board.history.push(hash);
    board.history.push(hash);
    board.history.push(hash);

    assert!(board.is_game_over());
    assert_eq!(board.game_result(), 3);
}

#[test]
fn test_unblockable_checks() {
    // White King at c5 (file 2, rank 4), Black Hawk at a3 (file 0, rank 2).
    // Hawk check: diagonal length 2.
    // If a White pawn is on b4 (file 1, rank 3), it cannot block the Hawk check.
    // Let's verify that a knight cannot block it either.
    let hfen = "12/12/12/12/12/12/12/2K9/1P10/h11/12/11k w - - 0 1";
    let board = Board::from_hfen(hfen).unwrap();

    // Verify check is active
    assert!(board.in_check());

    // Generate moves. Verify that any legal moves either move the king or capture the checker.
    // There are no moves that "block" the check.
    let moves = board.generate_moves();
    for m in moves.iter() {
        let dest = m.get_dest();
        let src = m.get_src();
        if src == board.king_sq(Player::White) {
            // King moves to escape
            assert_ne!(dest, SQ::make(2, 4), "King cannot stay on check square");
        } else {
            // Non-king moves must be captures of the checker (the Hawk at a3/SQ(24))
            assert_eq!(
                dest,
                SQ::make(0, 2),
                "Only legal non-king move is to capture the checker"
            );
        }
    }
}
