// SPDX-License-Identifier: GPL-3.0-or-later
// HyperChess Core — hyperchess-search
// File: crates/hyperchess-search/tests/rules_integration.rs
// Version: 1.0.0
// Copyright (c) 2026 HyperChess Developer Team

//! Rules-correctness tests exercised *through* a searcher — necessarily live
//! here (not in hyperchess-rules, which must not depend on search).
//! Carried over verbatim from kyrpy-hyperchess-rust's
//! src/hyperchess/src/board/mod.rs (hfen_consistency_tests +
//! engine_integrity_tests) during the Phase 1/3 extraction — see
//! docs/hyperchess-core-extraction-plan.md §12.

use hyperchess_rules::tools::Searcher;
use hyperchess_rules::Board;
use hyperchess_search::AlphaBetaSearcher;

#[test]
fn test_alphabeta_does_not_corrupt_board() {
    let mut board = Board::start_pos();
    let mut searcher = AlphaBetaSearcher::new();
    for ply in 0..10 {
        let hfen_before = board.get_hfen();
        let mv = searcher.best_move(&board, 3);
        let hfen_after_query = board.get_hfen();
        // Board must not be changed by best_move
        assert_eq!(
            hfen_before, hfen_after_query,
            "best_move mutated board at ply {}",
            ply
        );
        assert!(!mv.is_null(), "null move at ply {}", ply);
        board.apply_move(mv);
        let hfen_after_apply = board.get_hfen();
        println!(
            "ply={} uci={} hfen={}",
            ply,
            mv.stringify(),
            hfen_after_apply
        );
        // Fullmove should only increase after black moves (odd ply)
        // Extract fullmove from HFEN
        let fullmove: u16 = hfen_after_apply
            .split_whitespace()
            .nth(5)
            .unwrap_or("1")
            .parse()
            .unwrap_or(1);
        let expected_fm: u16 = (ply / 2 + 1) as u16 + if ply % 2 == 1 { 1 } else { 0 };
        assert_eq!(
            fullmove, expected_fm,
            "fullmove wrong at ply {}: got {} expected {}",
            ply, fullmove, expected_fm
        );
    }
}

#[test]
fn test_game25_exact_moves() {
    let mut board = Board::start_pos();
    let moves = ["b2f6", "a11a7", "k2g6"];
    for (ply, uci) in moves.iter().enumerate() {
        let all = board.generate_moves();
        let mv = all
            .iter()
            .find(|m| m.stringify() == *uci)
            .copied()
            .unwrap_or_else(|| panic!("move {} not found at ply {}", uci, ply));
        board.apply_move(mv);
        println!("ply={} uci={}\n  hfen={}", ply, uci, board.get_hfen());
    }
    let mut searcher = AlphaBetaSearcher::new();
    let hfen_before = board.get_hfen();
    let mv = searcher.best_move(&board, 2);
    let hfen_after_query = board.get_hfen();
    println!("ply=3 best_move={}", mv.stringify());
    assert_eq!(
        hfen_before, hfen_after_query,
        "best_move corrupted board at ply 3"
    );
    board.apply_move(mv);
    let result_hfen = board.get_hfen();
    println!("ply=3 after apply: {}", result_hfen);
    let fullmove: u16 = result_hfen
        .split_whitespace()
        .nth(5)
        .unwrap_or("1")
        .parse()
        .unwrap();
    assert_eq!(fullmove, 3, "fullmove should be 3, got {}", fullmove);
}
