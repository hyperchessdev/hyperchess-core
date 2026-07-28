// SPDX-License-Identifier: GPL-3.0-or-later
// HyperChess Core — hyperchess-search-cuda
// File: crates/hyperchess-search-cuda/src/cuda_backend.rs
// Version: 1.0.0
// Copyright (c) 2026 HyperChess Developer Team

//! Host-side CUDA backend: GPU batch position evaluation and GPU-assisted root search.
//!
//! The PTX is compiled at build time from `kernels/src/eval.rs` via `build.rs`.
//! The `batch_eval` kernel scores N positions in parallel on the GPU; its math
//! comes from the shared `hyperchess_eval` crate, so GPU scores are
//! byte-identical to `hyperchess_rules::tools::eval::evaluate_base`.
//!
//! Performance notes:
//! * The CUDA context is set current **once per thread** and a NON_BLOCKING
//!   stream is created **once per thread** (kept in thread-local storage), so
//!   Rayon workers don't pay context/stream setup on every call.
//! * The kernel launch block size is queried once and cached on the singleton.

use hyperchess_rules::board::Board;
use hyperchess_rules::core::piece_move::HyperMove;
use hyperchess_rules::core::score::VALUE_MATE;
use hyperchess_rules::Player;

use cust::context::CurrentContext;
use cust::prelude::*;

use std::cell::RefCell;
use std::sync::OnceLock;

static PTX: &str = include_str!(concat!(env!("OUT_DIR"), "/hyperchess_kernels.ptx"));

/// Largest depth for which `gpu_root_search` builds a full GPU-evaluated minimax
/// frontier. Beyond this the host-side frontier (one board per leaf) would grow
/// too large, so we defer to CPU alpha-beta (which is both stronger, thanks to
/// pruning, and bounded in memory).
const GPU_FRONTIER_MAX_DEPTH: u32 = 3;

// ── Board encoding ────────────────────────────────────────────────────────────

/// Encode a Board into a 144-byte array.
/// Each byte is the `Piece` enum discriminant for that square (0 = empty).
pub fn encode_board(board: &Board) -> [u8; 144] {
    let mut buf = [0u8; 144];
    for sq_idx in 0u8..144 {
        let piece = board.piece_at(hyperchess_rules::core::sq::SQ(sq_idx));
        buf[sq_idx as usize] = piece as u8;
    }
    buf
}

// ── CUDA context (process-lifetime singleton) ─────────────────────────────────

struct CudaState {
    ctx: Context,
    module: Module,
    /// Block size suggested for `batch_eval`, queried once at init.
    block_size: u32,
}

// SAFETY: cust Context/Module are effectively process-global after init.
unsafe impl Send for CudaState {}
unsafe impl Sync for CudaState {}

static CUDA: OnceLock<Result<CudaState, String>> = OnceLock::new();

// Per-thread NON_BLOCKING stream, created lazily on first use after the shared
// context is made current on that thread.
thread_local! {
    static THREAD_STREAM: RefCell<Option<Stream>> = const { RefCell::new(None) };
}

fn cuda_state() -> Result<&'static CudaState, String> {
    CUDA.get_or_init(|| {
        let ctx = cust::quick_init().map_err(|e| format!("CUDA init failed: {e}"))?;
        let module = Module::from_ptx(PTX, &[]).map_err(|e| format!("PTX load failed: {e}"))?;
        let kernel = module
            .get_function("batch_eval")
            .map_err(|e| format!("get_function: {e}"))?;
        let (_, block_size) = kernel
            .suggested_launch_configuration(0, 0.into())
            .map_err(|e| format!("launch config: {e}"))?;
        Ok(CudaState {
            ctx,
            module,
            block_size,
        })
    })
    .as_ref()
    .map_err(|e| e.clone())
}

/// Run `f` with this thread's persistent stream, creating it (and binding the
/// shared context to the thread) on first use.
fn with_thread_stream<R>(
    state: &CudaState,
    f: impl FnOnce(&Stream) -> Result<R, String>,
) -> Result<R, String> {
    THREAD_STREAM.with(|cell| {
        let mut slot = cell.borrow_mut();
        if slot.is_none() {
            CurrentContext::set_current(&state.ctx).map_err(|e| format!("ctx set_current: {e}"))?;
            let stream = Stream::new(StreamFlags::NON_BLOCKING, None)
                .map_err(|e| format!("Stream::new: {e}"))?;
            *slot = Some(stream);
        }
        f(slot.as_ref().unwrap())
    })
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Try to initialise CUDA. Returns the device name on success, None otherwise.
pub fn try_init_cuda() -> Option<String> {
    match cuda_state() {
        Err(e) => {
            eprintln!("[cuda_backend] init error: {e}");
            None
        }
        Ok(_) => match cust::device::Device::get_device(0) {
            Err(e) => {
                eprintln!("[cuda_backend] get_device error: {e}");
                None
            }
            Ok(dev) => match dev.name() {
                Err(e) => {
                    eprintln!("[cuda_backend] name error: {e}");
                    None
                }
                Ok(n) => Some(n),
            },
        },
    }
}

/// Batch-evaluate `boards` on GPU.
///
/// Returns a `Vec<i32>` of scores in the same order as input, each from the
/// **side-to-move's perspective** (positive = moving side ahead), matching
/// `hyperchess_rules::tools::eval::evaluate_base()`.
///
/// Falls back to returning an error string on any CUDA failure so the caller
/// can transparently fall back to CPU.
pub fn gpu_batch_eval(boards: &[Board]) -> Result<Vec<i32>, String> {
    let n = boards.len();
    if n == 0 {
        return Ok(vec![]);
    }

    let state = cuda_state()?;

    // Encode all boards into a flat u8 buffer (n * 144 bytes).
    let mut flat_boards = vec![0u8; n * 144];
    let mut side_to_move = vec![0u8; n];
    for (i, board) in boards.iter().enumerate() {
        let encoded = encode_board(board);
        flat_boards[i * 144..(i + 1) * 144].copy_from_slice(&encoded);
        side_to_move[i] = if board.turn() == Player::White { 0 } else { 1 };
    }

    let kernel = state
        .module
        .get_function("batch_eval")
        .map_err(|e| format!("get_function: {e}"))?;
    let block_size = state.block_size.max(1);
    let grid_size = (n as u32).div_ceil(block_size);

    with_thread_stream(state, |stream| {
        let boards_gpu = flat_boards
            .as_slice()
            .as_dbuf()
            .map_err(|e| format!("boards H2D: {e}"))?;
        let side_gpu = side_to_move
            .as_slice()
            .as_dbuf()
            .map_err(|e| format!("side H2D: {e}"))?;

        let mut scores_out = vec![0i32; n];
        let scores_gpu = scores_out
            .as_slice()
            .as_dbuf()
            .map_err(|e| format!("scores alloc: {e}"))?;

        unsafe {
            launch!(
                kernel<<<grid_size, block_size, 0, stream>>>(
                    boards_gpu.as_device_ptr(),
                    boards_gpu.len(),
                    side_gpu.as_device_ptr(),
                    side_gpu.len(),
                    scores_gpu.as_device_ptr(),
                    n as u32
                )
            )
            .map_err(|e| format!("launch: {e}"))?;
        }

        stream.synchronize().map_err(|e| format!("sync: {e}"))?;
        scores_gpu
            .copy_to(&mut scores_out)
            .map_err(|e| format!("D2H: {e}"))?;
        Ok(scores_out)
    })
}

// ── GPU-assisted root search (correct, single-batch minimax) ──────────────────

/// GPU-assisted root search.
///
/// * `depth <= 1`: apply every root move → batch-evaluate all children in one
///   kernel launch → pick the move that leaves the opponent worst.
/// * `2 <= depth <= GPU_FRONTIER_MAX_DEPTH`: build the *entire* minimax frontier
///   on the CPU, evaluate every leaf in a **single** GPU batch, and minimax-back
///   up the values. This is exact negamax (no pruning) and feeds the GPU one
///   large batch instead of the previous one-launch-per-node anti-pattern.
/// * deeper: defer to CPU alpha-beta (stronger via pruning, bounded memory).
///
/// Falls back to CPU alpha-beta on any CUDA error.
pub fn gpu_root_search(board: &Board, depth: u32) -> HyperMove {
    gpu_root_search_inner(board, depth).unwrap_or_else(|e| {
        eprintln!("[cuda] GPU search failed ({e}), falling back to CPU");
        cpu_alphabeta_move(board, depth)
    })
}

fn gpu_root_search_inner(board: &Board, depth: u32) -> Result<HyperMove, String> {
    let moves = board.generate_moves();
    if moves.is_empty() {
        return Ok(HyperMove::null());
    }

    if depth <= 1 {
        // One ply: GPU evaluates all children simultaneously.
        let children: Vec<Board> = moves
            .iter()
            .map(|&mv| {
                let mut b = board.clone();
                b.apply_move(mv);
                b
            })
            .collect();
        let scores = gpu_batch_eval(&children)?;
        // Child scores are from the child's (opponent's) perspective: minimise
        // the opponent = maximise -score.
        let best = moves
            .iter()
            .zip(scores.iter())
            .max_by_key(|(_, &s)| -s)
            .map(|(&mv, _)| mv)
            .unwrap_or(HyperMove::null());
        return Ok(best);
    }

    if depth > GPU_FRONTIER_MAX_DEPTH {
        // Too deep for a host-resident frontier — CPU alpha-beta is stronger here.
        return Ok(cpu_alphabeta_move(board, depth));
    }

    // Build the full minimax frontier, then evaluate every leaf in ONE batch.
    let mut frontier: Vec<Board> = Vec::new();
    let mut b = board.clone();
    let root_nodes: Vec<(HyperMove, MiniNode)> = moves
        .iter()
        .map(|&mv| {
            b.apply_move(mv);
            let node = build_frontier(&mut b, depth - 1, &mut frontier);
            b.undo_move();
            (mv, node)
        })
        .collect();

    let scores = gpu_batch_eval(&frontier)?;

    // Root value (our perspective) = max over moves of -value(child).
    let best = root_nodes
        .iter()
        .map(|(mv, node)| (*mv, -backup(node, &scores)))
        .max_by_key(|(_, v)| *v)
        .map(|(mv, _)| mv)
        .unwrap_or(HyperMove::null());
    Ok(best)
}

/// A node in the minimax frontier tree: a leaf to be GPU-evaluated, a precomputed
/// terminal (mate/draw) value, or an internal node with children.
enum MiniNode {
    /// Index into the frontier board list; value comes from the GPU batch.
    Eval(usize),
    /// Precomputed terminal value from this node's side-to-move perspective.
    Terminal(i32),
    Internal(Vec<MiniNode>),
}

/// Recursively expand `board` to depth `d`, recording leaf boards in `frontier`.
fn build_frontier(board: &mut Board, d: u32, frontier: &mut Vec<Board>) -> MiniNode {
    if d == 0 {
        let idx = frontier.len();
        frontier.push(board.clone());
        return MiniNode::Eval(idx);
    }

    let moves = board.generate_moves();
    if moves.is_empty() {
        // Checkmate (prefer faster mates via ply) or stalemate.
        let v = if board.in_check() {
            -VALUE_MATE + board.ply() as i32
        } else {
            0
        };
        return MiniNode::Terminal(v);
    }

    let mut children = Vec::with_capacity(moves.len());
    for m in moves.iter() {
        board.apply_move(*m);
        children.push(build_frontier(board, d - 1, frontier));
        board.undo_move();
    }
    MiniNode::Internal(children)
}

/// Negamax back-up over the frontier tree.
fn backup(node: &MiniNode, scores: &[i32]) -> i32 {
    match node {
        MiniNode::Eval(i) => scores[*i],
        MiniNode::Terminal(v) => *v,
        MiniNode::Internal(children) => children
            .iter()
            .map(|c| -backup(c, scores))
            .max()
            .unwrap_or(0),
    }
}

// ── CPU fallback ──────────────────────────────────────────────────────────────

/// Run the search entirely on the CPU.
///
/// The fallback taken whenever CUDA init or a kernel launch fails, so a machine
/// without a working GPU still plays — just slower.
pub fn cpu_alphabeta_move(board: &Board, depth: u32) -> HyperMove {
    use hyperchess_rules::tools::Searcher;
    use hyperchess_search::AlphaBetaSearcher;
    AlphaBetaSearcher::new().best_move(board, depth)
}
