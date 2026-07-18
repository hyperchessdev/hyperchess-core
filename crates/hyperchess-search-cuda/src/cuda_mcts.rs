//! GPU-accelerated MCTS for HyperChess.
//!
//! Strategy: **batched leaf evaluation**.
//!
//! Rather than calling CPU eval once per simulation (serial), we run
//! `BATCH_SIZE` SELECT → EXPAND steps without evaluating leaves, then send all
//! leaf positions to the GPU in a single `gpu_batch_eval` call, and finally
//! backpropagate all results.  This keeps the GPU fed with large batches and
//! amortises CUDA kernel launch overhead.
//!
//! When CUDA is unavailable (or the GPU call fails), falls back to CPU eval.

use std::sync::atomic::AtomicBool;

use hyperchess_rules::board::Board;
use hyperchess_rules::core::piece_move::HyperMove;
use hyperchess_rules::tools::eval::evaluate;
use hyperchess_search::mcts::mcts_with_eval_bounded;

#[cfg(feature = "cuda")]
use crate::cuda_backend::gpu_batch_eval;

/// Default number of simulations to batch per GPU kernel launch. Larger batches
/// amortise launch/transfer overhead and keep more of the A6000's SMs busy.
#[allow(dead_code)]
pub const DEFAULT_BATCH_SIZE: usize = 1024;

/// GPU-batched MCTS with no time bound — full simulation budget.
///
/// Prefer [`mcts_cuda_bounded`] anywhere a wall-clock budget or external stop
/// applies (web API, CLI move timeouts).
pub fn mcts_cuda(board: &Board, simulations: u32, batch_size: usize) -> HyperMove {
    mcts_cuda_bounded(board, simulations, batch_size, 0, &AtomicBool::new(false))
}

/// GPU-batched MCTS honouring the same bounds as the CPU paths: `movetime_ms`
/// is a hard wall-clock budget (`0` = no clock) and `stop` may be flipped from
/// another thread. Both are consulted between GPU batches.
///
/// Delegates to the shared [`mcts_with_eval_bounded`] driver, so batched
/// selection carries **virtual loss** exactly like the CPU implementation —
/// selections within one batch explore distinct paths instead of repeatedly
/// descending the same UCB-best line.
pub fn mcts_cuda_bounded(
    board: &Board,
    simulations: u32,
    batch_size: usize,
    movetime_ms: u64,
    stop: &AtomicBool,
) -> HyperMove {
    mcts_with_eval_bounded(board, simulations, batch_size, movetime_ms, stop, |batch| {
        let boards: Vec<Board> = batch.iter().map(|(_, b)| b.clone()).collect();
        eval_batch(&boards)
    })
}

// ── Leaf evaluation dispatcher ────────────────────────────────────────────────

/// Evaluate a batch of boards, returning scores in [-1, 1] from each
/// board's side-to-move perspective.
///
/// Uses GPU batch eval when the `cuda` feature is compiled in; falls back
/// to per-position CPU eval otherwise.
fn eval_batch(boards: &[Board]) -> Vec<f64> {
    #[cfg(feature = "cuda")]
    {
        match gpu_batch_eval(boards) {
            Ok(scores) => {
                return scores
                    .into_iter()
                    .map(|s| (s as f64).clamp(-3000.0, 3000.0) / 3000.0)
                    .collect();
            }
            Err(e) => {
                eprintln!("[cuda_mcts] GPU eval failed: {e}; falling back to CPU");
            }
        }
    }

    // CPU fallback
    boards
        .iter()
        .map(|b| (evaluate(b) as f64).clamp(-3000.0, 3000.0) / 3000.0)
        .collect()
}

// ── Parallel MCTS (root-parallel, each thread uses GPU batch eval) ────────────

/// Root-parallel MCTS: launch one tree per Rayon thread, combine vote counts.
///
/// This is the primary path for CUDA MCTS — Rayon keeps all GPU compute units
/// busy by running multiple independent MCTS trees simultaneously.
pub fn mcts_cuda_parallel(
    board: &Board,
    simulations: u32,
    threads: usize,
    batch_size: usize,
) -> HyperMove {
    mcts_cuda_parallel_bounded(
        board,
        simulations,
        threads,
        batch_size,
        0,
        &AtomicBool::new(false),
    )
}

/// [`mcts_cuda_parallel`] with a wall-clock budget and external stop flag.
/// All trees share the same deadline and `stop`, so the whole ensemble
/// returns within the budget.
pub fn mcts_cuda_parallel_bounded(
    board: &Board,
    simulations: u32,
    threads: usize,
    batch_size: usize,
    movetime_ms: u64,
    stop: &AtomicBool,
) -> HyperMove {
    use rayon::prelude::*;

    if simulations == 0 {
        return board
            .generate_moves()
            .iter()
            .next()
            .copied()
            .unwrap_or(HyperMove::null());
    }

    let active_threads = threads.max(1).min(simulations as usize);
    let base = simulations as usize / active_threads;
    let rem = simulations as usize % active_threads;

    // Each thread runs its own MCTS tree with GPU batch eval
    let votes: Vec<HyperMove> = (0..active_threads)
        .collect::<Vec<_>>()
        .par_iter()
        .map(|&i| {
            let sims = base + usize::from(i < rem);
            mcts_cuda_bounded(board, sims as u32, batch_size, movetime_ms, stop)
        })
        .collect();

    // Pick root move that wins the most votes
    let moves = board.generate_moves();
    let mut tally: std::collections::HashMap<u32, u32> = std::collections::HashMap::new();
    for mv in votes {
        if !mv.is_null() {
            *tally.entry(mv.get_raw()).or_insert(0) += 1;
        }
    }

    moves
        .iter()
        .max_by_key(|m| tally.get(&m.get_raw()).copied().unwrap_or(0))
        .copied()
        .unwrap_or(HyperMove::null())
}
