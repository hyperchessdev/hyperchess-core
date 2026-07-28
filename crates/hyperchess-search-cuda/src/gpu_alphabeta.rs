// SPDX-License-Identifier: GPL-3.0-or-later
// HyperChess Core — hyperchess-search-cuda
// File: crates/hyperchess-search-cuda/src/gpu_alphabeta.rs
// Version: 1.0.0
// Copyright (c) 2026 HyperChess Developer Team

//! GPU-guided alpha-beta and iterative deepening.
//!
//! Strategy: at *shallow* internal nodes (remaining depth ≥
//! [`GPU_ORDER_MIN_DEPTH`]) all child positions are batch-evaluated on GPU in a
//! single kernel launch, and the GPU scores replace MVV-LVA for move ordering.
//! Deeper interior nodes use the canonical cheap tactical ordering
//! (MVV-LVA + TT move) — cloning every child board and paying a kernel launch
//! per node is far slower than the search it is meant to speed up, so GPU
//! ordering is reserved for the nodes where a good ordering matters most.
//!
//! Rules and scoring mirror the canonical CPU search
//! ([`hyperchess_search::TimedSearcher`]):
//!
//! * repetition / 224-half-move / insufficient-material draws
//! * check extensions
//! * capture-only **quiescence** at the horizon (CPU eval — resolving tactics
//!   serially on GPU would be slower than not using the GPU at all)
//! * TT mate scores stored/probed through [`value_to_tt`]/[`value_from_tt`]
//! * wall-clock / external-stop cancellation via the `_bounded` entry points
//!
//! Falls back to CPU eval transparently on any CUDA error.

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use hyperchess_rules::board::Board;
use hyperchess_rules::core::piece_move::HyperMove;
use hyperchess_rules::core::score::*;
use hyperchess_rules::tools::eval::evaluate;
use hyperchess_rules::tools::tt::{TTFlag, TranspositionTable};
use hyperchess_rules::MoveList;
use hyperchess_search::search::{
    order_tactical, promote_tt_move, quiesce, terminal_value, value_from_tt, value_to_tt,
};

use crate::cuda_backend::gpu_batch_eval;

/// GPU batch ordering only at nodes with at least this much remaining depth.
/// Below it, the per-node board clones + kernel launch cost more than the
/// ordering gain; cheap MVV-LVA/TT ordering is used instead.
const GPU_ORDER_MIN_DEPTH: i32 = 3;
/// How often (in nodes) the stop flag / deadline are consulted.
const CHECK_INTERVAL: u64 = 2048;

// ── Public entry points ───────────────────────────────────────────────────────

/// GPU-guided alpha-beta search with no time bound. Returns the best move.
pub fn gpu_alphabeta_search(board: &Board, depth: u32) -> HyperMove {
    gpu_alphabeta_search_bounded(board, depth, 0, &AtomicBool::new(false))
}

/// [`gpu_alphabeta_search`] with a hard wall-clock budget (`0` = no clock) and
/// an external stop flag, both consulted inside the recursive search.
pub fn gpu_alphabeta_search_bounded(
    board: &Board,
    depth: u32,
    movetime_ms: u64,
    stop: &AtomicBool,
) -> HyperMove {
    let mut b = board.clone();
    let mut ctx = GpuCtx::new(movetime_ms, stop);
    let (_, mv) = gpu_search(
        &mut b,
        &mut None,
        &mut ctx,
        depth as i32,
        -VALUE_INFINITE,
        VALUE_INFINITE,
        0,
    );
    legal_or_fallback(&b, mv)
}

/// GPU-guided iterative-deepening search with a transposition table and no
/// time bound.
pub fn gpu_iterative_search(board: &Board, max_depth: u32) -> HyperMove {
    gpu_iterative_search_bounded(board, max_depth, 0, &AtomicBool::new(false))
}

/// [`gpu_iterative_search`] with a hard wall-clock budget (`0` = no clock) and
/// an external stop flag. Anytime: returns the best move from the deepest
/// *completed* depth when a budget triggers mid-iteration.
pub fn gpu_iterative_search_bounded(
    board: &Board,
    max_depth: u32,
    movetime_ms: u64,
    stop: &AtomicBool,
) -> HyperMove {
    let mut b = board.clone();
    let mut tt = TranspositionTable::new(1 << 20);
    let mut ctx = GpuCtx::new(movetime_ms, stop);
    let mut best = HyperMove::null();

    for depth in 1..=max_depth {
        if ctx.budget_exhausted() {
            break;
        }
        let (_, mv) = gpu_search(
            &mut b,
            &mut Some(&mut tt),
            &mut ctx,
            depth as i32,
            -VALUE_INFINITE,
            VALUE_INFINITE,
            0,
        );
        if ctx.aborted {
            break; // keep the move from the last completed depth
        }
        if !mv.is_null() {
            best = mv;
        }
    }

    legal_or_fallback(&b, best)
}

// ── Search context (budget / cancellation) ────────────────────────────────────

struct GpuCtx<'a> {
    deadline: Option<Instant>,
    stop: &'a AtomicBool,
    nodes: u64,
    aborted: bool,
}

impl<'a> GpuCtx<'a> {
    fn new(movetime_ms: u64, stop: &'a AtomicBool) -> Self {
        Self {
            deadline: (movetime_ms > 0)
                .then(|| Instant::now() + Duration::from_millis(movetime_ms)),
            stop,
            nodes: 0,
            aborted: false,
        }
    }

    /// Count a node; every `CHECK_INTERVAL` nodes consult the stop flag and
    /// deadline. Once `aborted` is set it stays set.
    #[inline]
    fn tick(&mut self) {
        self.nodes += 1;
        if self.nodes % CHECK_INTERVAL == 0 && self.budget_exhausted() {
            self.aborted = true;
        }
    }

    #[inline]
    fn budget_exhausted(&self) -> bool {
        if self.aborted || self.stop.load(Ordering::Relaxed) {
            return true;
        }
        match self.deadline {
            Some(d) => Instant::now() >= d,
            None => false,
        }
    }
}

// ── Internal search ───────────────────────────────────────────────────────────

/// Negamax with optional TT. Rules/scoring mirror the canonical CPU search;
/// see the module docs for the parity list.
fn gpu_search(
    board: &mut Board,
    tt: &mut Option<&mut TranspositionTable>,
    ctx: &mut GpuCtx,
    depth: i32,
    mut alpha: Value,
    beta: Value,
    ply: usize,
) -> (Value, HyperMove) {
    if ctx.aborted {
        return (alpha, HyperMove::null());
    }
    ctx.tick();
    if ctx.aborted {
        return (alpha, HyperMove::null());
    }

    let in_check = board.in_check();
    let ply_val = ply as Value;

    // Draw rules, mirroring `TimedSearcher`: two prior occurrences plus the
    // current position is threefold; the scaled 50-move rule (224 half-moves on
    // 12×12) and insufficient material. The root is exempt so a move is always
    // produced; in check the move rule only counts when an evasion exists.
    if ply > 0 {
        if board.repetition_count() >= 2 {
            return (VALUE_DRAW, HyperMove::null());
        }
        if board.state.rule50 >= 224 && (!in_check || !board.generate_moves().is_empty()) {
            return (VALUE_DRAW, HyperMove::null());
        }
        if board.insufficient_material() {
            return (VALUE_DRAW, HyperMove::null());
        }
    }

    // Check extension, bounded by the repetition draw and the budget.
    let depth = if in_check { depth + 1 } else { depth };

    if depth <= 0 {
        let score = quiesce(board, alpha, beta, ply, false, &mut || {
            ctx.tick();
            ctx.aborted
        });
        return (score, HyperMove::null());
    }

    let key = board.state.zobrist;

    // TT probe. Mate scores are re-projected onto this node's ply scale; no TT
    // cutoffs at the root so a key collision can never inject an illegal move.
    let mut tt_move = HyperMove::null();
    if let Some(t) = tt.as_deref_mut() {
        if let Some(entry) = t.probe(key) {
            tt_move = entry.best_move;
            if entry.depth >= depth && ply > 0 {
                let tt_score = value_from_tt(entry.score, ply_val);
                match entry.flag {
                    TTFlag::Exact => return (tt_score, entry.best_move),
                    TTFlag::LowerBound => {
                        if tt_score >= beta {
                            return (tt_score, entry.best_move);
                        }
                    }
                    TTFlag::UpperBound => {
                        if tt_score <= alpha {
                            return (tt_score, entry.best_move);
                        }
                    }
                }
            }
        }
    }

    let moves = board.generate_moves();
    if moves.is_empty() {
        return (terminal_value(board, ply_val), HyperMove::null());
    }

    // GPU-guided ordering only where the kernel launch pays for itself; cheap
    // tactical (MVV-LVA + TT move) ordering at deeper interior nodes.
    let mut ordered = if depth >= GPU_ORDER_MIN_DEPTH {
        gpu_order_moves(board, &moves)
    } else {
        order_tactical(board, &moves, tt_move)
    };
    promote_tt_move(&mut ordered, tt_move);

    let mut best_move = ordered[0].0;
    let mut best_score = -VALUE_INFINITE;
    let mut flag = TTFlag::UpperBound;

    for (mv, _) in &ordered {
        board.apply_move(*mv);
        let (score, _) = gpu_search(board, tt, ctx, depth - 1, -beta, -alpha, ply + 1);
        let score = -score;
        board.undo_move();

        if ctx.aborted {
            // Partial iteration: report what we have without storing a TT node.
            return (best_score.max(alpha), best_move);
        }

        if score > best_score {
            best_score = score;
            best_move = *mv;
        }
        if score > alpha {
            alpha = score;
            flag = TTFlag::Exact;
            if alpha >= beta {
                flag = TTFlag::LowerBound;
                break;
            }
        }
    }

    if let Some(t) = tt.as_deref_mut() {
        t.store(
            key,
            best_move,
            value_to_tt(best_score, ply_val),
            depth,
            flag,
        );
    }
    (best_score, best_move)
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Return `mv` if non-null, otherwise the first legal move (or null when the
/// position is terminal) — the public entry points always produce a legal move
/// when one exists.
fn legal_or_fallback(board: &Board, mv: HyperMove) -> HyperMove {
    if !mv.is_null() {
        return mv;
    }
    board
        .generate_moves()
        .iter()
        .next()
        .copied()
        .unwrap_or(HyperMove::null())
}

/// Batch-evaluate all children of `board` on GPU and return moves sorted
/// best-first from the current side-to-move's perspective.
fn gpu_order_moves(board: &mut Board, moves: &MoveList) -> Vec<(HyperMove, i32)> {
    let mut child_boards: Vec<Board> = Vec::with_capacity(moves.len());
    for &mv in moves.iter() {
        let mut b = board.clone();
        b.apply_move(mv);
        child_boards.push(b);
    }

    // GPU scores are from each child's perspective; negate to score from our side.
    let scores = gpu_batch_eval(&child_boards)
        .unwrap_or_else(|_| child_boards.iter().map(|b| evaluate(b)).collect());

    let mut result: Vec<(HyperMove, i32)> = moves
        .iter()
        .zip(scores.iter())
        .map(|(&mv, &s)| (mv, -s))
        .collect();

    result.sort_unstable_by(|a, b| b.1.cmp(&a.1));
    result
}
