// SPDX-License-Identifier: GPL-3.0-or-later
// HyperChess Core — hyperchess-rules
// File: crates/hyperchess-rules/tests/eval_ab.rs
// Version: 1.0.0
// Copyright (c) 2026 HyperChess Developer Team

//! A/B self-play: full `evaluate` (material+PSQT+phase + mobility/king-safety)
//! vs `evaluate_base` (material+PSQT+phase only). Same fixed-depth negamax for
//! both sides, so the only variable is the eval function — this isolates whether
//! the hand-crafted terms actually improve play.
//!
//! Run with: `cargo test -p hyperchess --test eval_ab --release -- --ignored --nocapture`

use hyperchess_rules::board::Board;
use hyperchess_rules::tools::eval::{evaluate, evaluate_base, evaluate_variant};
use hyperchess_rules::{HyperMove, Player};

type EvalFn = fn(&Board) -> i32;

/// Full eval including the threats term (== `evaluate`).
fn full_with_threats(b: &Board) -> i32 {
    evaluate_variant(b, true)
}
/// Full eval minus the threats term (mobility + king-safety only).
fn full_no_threats(b: &Board) -> i32 {
    evaluate_variant(b, false)
}

const INF: i32 = 1_000_000;

/// Plain fixed-depth negamax with alpha-beta. Eval is from the side-to-move
/// perspective (both eval fns already are), so this is a clean negamax.
fn negamax(board: &Board, depth: u32, mut alpha: i32, beta: i32, eval: EvalFn) -> i32 {
    if board.is_game_over() {
        // Side to move has no move: checkmate (bad) or stalemate (0). Use eval
        // sign convention: large negative if lost.
        return eval(board);
    }
    if depth == 0 {
        return eval(board);
    }
    let moves = board.generate_moves();
    if moves.is_empty() {
        return eval(board);
    }
    let mut best = -INF;
    for m in moves.iter() {
        let mut next = board.clone();
        next.apply_move(*m);
        let score = -negamax(&next, depth - 1, -beta, -alpha, eval);
        if score > best {
            best = score;
        }
        if best > alpha {
            alpha = best;
        }
        if alpha >= beta {
            break;
        }
    }
    best
}

fn pick_move(board: &Board, depth: u32, eval: EvalFn, tie_break: usize) -> Option<HyperMove> {
    let moves = board.generate_moves();
    if moves.is_empty() {
        return None;
    }
    let mut best_score = -INF;
    let mut best: Option<HyperMove> = None;
    for (i, m) in moves.iter().enumerate() {
        let mut next = board.clone();
        next.apply_move(*m);
        let score = -negamax(&next, depth - 1, -INF, INF, eval);
        // Deterministic tie-break that still varies game-to-game via `tie_break`.
        if score > best_score || (score == best_score && i % 7 == tie_break % 7) {
            best_score = score;
            best = Some(*m);
        }
    }
    best.or_else(|| moves.iter().next().copied())
}

/// Play one game. White uses `white_eval`, Black uses `black_eval`.
/// Returns 1 (white win), -1 (black win), 0 (draw/other).
fn play_game(white_eval: EvalFn, black_eval: EvalFn, depth: u32, opening: usize) -> i32 {
    let mut board = Board::start_pos();
    // Diversify openings deterministically: first 2 plies pick the Nth legal move.
    for ply in 0..2 {
        let moves = board.generate_moves();
        if moves.is_empty() {
            break;
        }
        let idx = (opening + ply) % moves.len();
        let m = moves.iter().nth(idx).copied().unwrap();
        board.apply_move(m);
    }
    for _ in 0..120 {
        if board.is_game_over() {
            break;
        }
        let eval = if board.turn() == Player::White {
            white_eval
        } else {
            black_eval
        };
        match pick_move(&board, depth, eval, opening) {
            Some(m) => board.apply_move(m),
            None => break,
        }
    }
    match board.game_result() {
        1 => 1,
        2 => -1,
        _ => 0,
    }
}

#[test]
#[ignore = "slow self-play tournament; run with --release --ignored --nocapture"]
fn eval_ab_tournament() {
    let depth = 2;
    let n_openings = 12;
    let (mut full_wins, mut base_wins, mut draws) = (0, 0, 0);

    for o in 0..n_openings {
        // Game A: full eval = White, base eval = Black
        match play_game(evaluate, evaluate_base, depth, o) {
            1 => full_wins += 1,
            -1 => base_wins += 1,
            _ => draws += 1,
        }
        // Game B: colors swapped to cancel first-move advantage
        match play_game(evaluate_base, evaluate, depth, o) {
            1 => base_wins += 1,
            -1 => full_wins += 1,
            _ => draws += 1,
        }
    }

    let total = full_wins + base_wins + draws;
    let decisive = full_wins + base_wins;
    println!(
        "Eval A/B over {} games (depth {}): full={} base={} draws={}",
        total, depth, full_wins, base_wins, draws
    );
    if decisive > 0 {
        println!(
            "Full-eval score among decisive games: {:.1}%",
            100.0 * full_wins as f64 / decisive as f64
        );
    }
    // The terms must not make play *worse*: full eval should not lose the match.
    assert!(
        full_wins >= base_wins,
        "mobility/king-safety terms regressed play: full={} base={}",
        full_wins,
        base_wins
    );
}

/// Isolates the threats term: full eval (with threats) vs the same eval without
/// it (mobility + king-safety only). Confirms the threats term does not regress.
#[test]
#[ignore = "slow self-play tournament; run with --release --ignored --nocapture"]
fn threats_ab_tournament() {
    let depth = 2;
    let n_openings = 12;
    let (mut thr_wins, mut no_wins, mut draws) = (0, 0, 0);

    for o in 0..n_openings {
        match play_game(full_with_threats, full_no_threats, depth, o) {
            1 => thr_wins += 1,
            -1 => no_wins += 1,
            _ => draws += 1,
        }
        match play_game(full_no_threats, full_with_threats, depth, o) {
            1 => no_wins += 1,
            -1 => thr_wins += 1,
            _ => draws += 1,
        }
    }

    let total = thr_wins + no_wins + draws;
    let decisive = thr_wins + no_wins;
    println!(
        "Threats A/B over {} games (depth {}): with_threats={} no_threats={} draws={}",
        total, depth, thr_wins, no_wins, draws
    );
    if decisive > 0 {
        println!(
            "With-threats score among decisive games: {:.1}%",
            100.0 * thr_wins as f64 / decisive as f64
        );
    }
    // Threats must not regress play.
    assert!(
        thr_wins >= no_wins,
        "threats term regressed play: with={} without={}",
        thr_wins,
        no_wins
    );
}
