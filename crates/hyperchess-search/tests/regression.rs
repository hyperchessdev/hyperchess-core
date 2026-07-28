// SPDX-License-Identifier: GPL-3.0-or-later
// HyperChess Core — hyperchess-search
// File: crates/hyperchess-search/tests/regression.rs
// Version: 1.0.0
// Copyright (c) 2026 HyperChess Developer Team

//! Phase-0 regression safety net (see refactor blueprint).
//!
//! These golden values lock the engine's observable behavior *before* the large
//! refactor so later mechanical steps can prove "no change", and intentional
//! behavior changes (e.g. the mate-score TT fix, GPU eval unification) show up
//! as deliberate fixture updates rather than silent regressions.
//!
//! Run fast subset:   cargo test -p hyperchess-search --test regression
//! Run everything:    cargo test -p hyperchess-search --release --test regression -- --include-ignored

use hyperchess_rules::board::perft::perft;
use hyperchess_rules::tools::eval::evaluate;
use hyperchess_rules::tools::Searcher;
use hyperchess_rules::Board;
use hyperchess_search::{
    AlphaBetaSearcher, GuidedAlphaBeta, GuidedIterative, IterativeSearcher, StrategicSearcher,
};

const P4_FEN: &str =
    "12/abcdefghijkl/mnopqrstuvwx/12/12/12/12/12/12/MNOPQRSTUVWX/ABCDEF1HIJKL/4G7 w - - 4 3";
const P8_FEN: &str =
    "12/abcdefghijkl/mnopqrstuvwx/12/12/12/12/12/12/MNOPQRSTUVWX/ABCDEF1HIJKL/2G9 w - - 8 5";

/// Advance by applying the first generated move `n` times (movegen-deterministic).
fn walk(n: usize) -> Board {
    let mut b = Board::start_pos();
    for _ in 0..n {
        let moves = b.generate_moves();
        if moves.is_empty() {
            break;
        }
        b.apply_move(moves.get(0));
    }
    b
}

#[test]
fn perft_start_golden() {
    let mut b = Board::start_pos();
    assert_eq!(perft(&mut b, 1), 62);
    assert_eq!(perft(&mut b, 2), 3844);
    assert_eq!(perft(&mut b, 3), 249608);
}

#[test]
#[ignore = "slow (16M nodes); run in release with --include-ignored"]
fn perft_start_depth4_golden() {
    let mut b = Board::start_pos();
    assert_eq!(perft(&mut b, 4), 16188073);
}

#[test]
fn perft_positions_golden() {
    // P4 and P8 are reproducible non-start positions (first-move walk).
    let mut p4 = Board::from_hfen(P4_FEN).unwrap();
    assert_eq!(
        walk(4).get_hfen(),
        P4_FEN,
        "P4 HFEN drift — movegen order changed"
    );
    assert_eq!(perft(&mut p4, 1), 61);
    assert_eq!(perft(&mut p4, 2), 3782);
    assert_eq!(perft(&mut p4, 3), 240556);

    let mut p8 = Board::from_hfen(P8_FEN).unwrap();
    assert_eq!(
        walk(8).get_hfen(),
        P8_FEN,
        "P8 HFEN drift — movegen order changed"
    );
    assert_eq!(perft(&mut p8, 1), 60);
    assert_eq!(perft(&mut p8, 2), 3720);
    assert_eq!(perft(&mut p8, 3), 233922);
}

#[test]
fn eval_golden() {
    // `evaluate` is base (material+PSQT+phase) + mobility/king-safety terms.
    // Start + P4 are mirror-symmetric -> exactly 0; P8 (white king on c-file,
    // off its mirror square) shifts from the base 15 to 13 under the terms.
    assert_eq!(evaluate(&Board::start_pos()), 0);
    assert_eq!(evaluate(&Board::from_hfen(P4_FEN).unwrap()), 0);
    assert_eq!(evaluate(&Board::from_hfen(P8_FEN).unwrap()), 13);
}

/// All alpha-beta-family searcher names now resolve to the single canonical
/// `TimedSearcher`, so at a fixed depth they must return the **same legal move**.
/// This is the unification invariant; MCTS is excluded (intentionally randomized).
#[test]
fn searcher_unification_golden() {
    for (fen_name, b) in [
        ("start", Board::start_pos()),
        ("P4", Board::from_hfen(P4_FEN).unwrap()),
        ("P8", Board::from_hfen(P8_FEN).unwrap()),
    ] {
        let legal: Vec<String> = b.generate_moves().iter().map(|m| m.stringify()).collect();

        let ab = AlphaBetaSearcher::new().best_move(&b, 3).stringify();
        let id = IterativeSearcher::new().best_move(&b, 3).stringify();
        let gab = GuidedAlphaBeta::new().best_move(&b, 3).stringify();
        let gid = GuidedIterative::new().best_move(&b, 3).stringify();
        let strategic = StrategicSearcher::new().best_move(&b, 3).stringify();

        assert!(
            legal.contains(&ab),
            "AB returned illegal move {ab} ({fen_name})"
        );
        assert!(
            legal.contains(&strategic),
            "Strategic returned illegal move {strategic} ({fen_name})"
        );
        assert_eq!(ab, id, "AB vs ID disagree ({fen_name})");
        assert_eq!(ab, gab, "AB vs GAB disagree ({fen_name})");
        assert_eq!(ab, gid, "AB vs GID disagree ({fen_name})");
    }
}
