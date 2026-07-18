//! CUDA search budget/contract tests (compiled only with `--features cuda`).
//!
//! These verify the bounded GPU entry points honour wall-clock budgets and
//! external stop flags — the CPU paths have equivalent unit tests, and the
//! historical bug was precisely that the CUDA paths accepted no bounds at all.
//! `gpu_batch_eval` falls back to CPU eval on any CUDA error, so the tests are
//! meaningful (though slower) even when no GPU is present.
#![cfg(feature = "cuda")]

use std::sync::atomic::AtomicBool;
use std::time::{Duration, Instant};

use hyperchess_rules::Board;
use hyperchess_search_cuda::cuda_mcts::{mcts_cuda_bounded, mcts_cuda_parallel_bounded};
use hyperchess_search_cuda::gpu_alphabeta::gpu_iterative_search_bounded;

fn assert_legal(board: &Board, mv: hyperchess_rules::core::piece_move::HyperMove) {
    assert!(!mv.is_null());
    assert!(board.generate_moves().iter().any(|&m| m == mv));
}

#[test]
fn cuda_mcts_preset_stop_returns_a_legal_move_immediately() {
    hyperchess_rules::Helper::init();
    let board = Board::start_pos();
    let stop = AtomicBool::new(true); // already stopped
    let start = Instant::now();
    let mv = mcts_cuda_bounded(&board, 10_000_000, 512, 0, &stop);
    assert!(start.elapsed() < Duration::from_secs(10));
    assert_legal(&board, mv);
}

#[test]
fn cuda_mcts_movetime_truncates_a_huge_simulation_budget() {
    hyperchess_rules::Helper::init();
    let board = Board::start_pos();
    let stop = AtomicBool::new(false);
    let start = Instant::now();
    // 200 ms budget vs. a budget of 10M simulations: the deadline check between
    // batches must cut the loop short. Generous bound to stay CI-safe.
    let mv = mcts_cuda_bounded(&board, 10_000_000, 512, 200, &stop);
    assert!(
        start.elapsed() < Duration::from_secs(30),
        "movetime did not bound CUDA MCTS (took {:?})",
        start.elapsed()
    );
    assert_legal(&board, mv);
}

#[test]
fn cuda_mcts_parallel_movetime_truncates() {
    hyperchess_rules::Helper::init();
    let board = Board::start_pos();
    let stop = AtomicBool::new(false);
    let start = Instant::now();
    let mv = mcts_cuda_parallel_bounded(&board, 10_000_000, 4, 512, 200, &stop);
    assert!(
        start.elapsed() < Duration::from_secs(30),
        "movetime did not bound parallel CUDA MCTS (took {:?})",
        start.elapsed()
    );
    assert_legal(&board, mv);
}

#[test]
fn gpu_iterative_movetime_truncates_a_deep_search() {
    hyperchess_rules::Helper::init();
    let board = Board::start_pos();
    let stop = AtomicBool::new(false);
    let start = Instant::now();
    let mv = gpu_iterative_search_bounded(&board, 64, 300, &stop);
    assert!(
        start.elapsed() < Duration::from_secs(30),
        "movetime did not bound GPU iterative search (took {:?})",
        start.elapsed()
    );
    assert_legal(&board, mv);
}

#[test]
fn gpu_iterative_finds_mate_in_one() {
    hyperchess_rules::Helper::init();
    // White: Qb11 mates on b12 supported by the king — same fixture family as
    // the CPU tests: back-rank style mate with only K+Q vs K.
    let board = Board::from_hfen("k11/1Q10/1K10/12/12/12/12/12/12/12/12/12 w - - 0 1")
        .expect("valid mate-in-one HFEN");
    let mv = gpu_iterative_search_bounded(&board, 4, 0, &AtomicBool::new(false));
    let mut b = board.clone();
    assert_legal(&board, mv);
    b.apply_move(mv);
    assert!(
        b.generate_moves().is_empty() && b.in_check(),
        "expected mate after {mv}, got HFEN {}",
        b.get_hfen()
    );
}
