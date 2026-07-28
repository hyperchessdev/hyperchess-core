//! Engine-vs-engine game loop — independent engine selection per side.

use std::time::Instant;

use hyperchess_rules::board::Board;
use hyperchess_rules::core::piece_move::HyperMove;
use hyperchess_rules::tools::eval;
use hyperchess_rules::Player;
use hyperchess_search::{SearchLimits, TimedSearcher};
use rand::{rngs::SmallRng, SeedableRng};

use super::export::{self, GameStats, MoveRecord};

#[cfg(feature = "cuda")]
use hyperchess_search_cuda::cuda_backend;
#[cfg(feature = "cuda")]
use hyperchess_search_cuda::cuda_mcts;

// ── GPU detection ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum GpuBackend {
    #[allow(dead_code)]
    Cuda(String),
    None,
}

pub fn detect_gpu() -> GpuBackend {
    #[cfg(feature = "cuda")]
    {
        if let Some(name) = cuda_backend::try_init_cuda() {
            return GpuBackend::Cuda(name);
        }
    }
    GpuBackend::None
}

// ── Per-side engine configuration ─────────────────────────────────────────────

/// All parameters needed to run one side's engine for a single move.
#[derive(Clone, Debug)]
pub struct EngineConfig {
    pub name: String,
    pub depth: u32,
    pub simulations: u32,
    #[allow(dead_code)]
    pub batch_size: usize,
    pub threads: usize,
}

impl EngineConfig {
    pub fn new(
        name: &str,
        depth: u32,
        simulations: u32,
        batch_size: usize,
        threads: usize,
    ) -> Self {
        EngineConfig {
            name: name.to_string(),
            depth,
            simulations,
            batch_size,
            threads,
        }
    }

    /// Human-readable label for export (e.g. "GPU-MCTS(800)" or "AlphaBeta(d4)").
    pub fn label(&self, gpu: &GpuBackend) -> String {
        match self.name.to_lowercase().as_str() {
            "cuda_mcts" => {
                let prefix = match gpu {
                    GpuBackend::Cuda(_) => "GPU-MCTS",
                    _ => "CPU-MCTS",
                };
                format!("{}({})", prefix, self.simulations)
            }
            "mcts" => format!("CPU-MCTS({})", self.simulations),
            "alphabeta" | "ab" => format!("CPU-AB(d{})", self.depth),
            "iterative" | "id" => format!("CPU-ID(d{})", self.depth),
            "guided" | "guided_ab" => format!("CPU-GAB(d{})", self.depth),
            "guided_id" => format!("CPU-GID(d{})", self.depth),
            "strategic" | "strategic" | "strategic_like" => {
                format!("CPU-Strategic(d{})", self.depth)
            }
            "aggressive" | "commercial" | "stockfish_like" => format!("CPU-Aggressive(d{})", self.depth),
            "random" => "Random".to_string(),
            other => other.to_string(),
        }
    }
}

// ── Move selection ─────────────────────────────────────────────────────────────

// ── Timeout helpers ───────────────────────────────────────────────────────────

/// Spawn `f` on a background thread; return its result or `None` if it does not
/// finish within `timeout_ms` milliseconds.  The background thread is not killed
/// on timeout — it runs to completion and its result is discarded.
///
/// Only the GPU (cuda_mcts) path still needs this coarse timeout; the CPU search is
/// natively anytime via `TimedSearcher`, so the wrapper is CUDA-gated.
#[cfg(feature = "cuda")]
fn try_with_timeout<F, R>(f: F, timeout_ms: u64) -> Option<R>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(f());
    });
    rx.recv_timeout(std::time::Duration::from_millis(timeout_ms))
        .ok()
}

/// AB search with a hard wall-clock budget.
/// Returns `(move, depth_ceiling)`.
///
/// Uses the canonical anytime [`TimedSearcher`]: it iteratively deepens up to
/// `depth` and returns the best move from the deepest *completed* iteration the
/// instant `timeout_ms` elapses. No restart-at-lower-depth and no leaked worker
/// thread (the old `try_with_timeout` spawned a thread that kept running after a
/// timeout) — the search watches the clock itself.
fn ab_with_timeout(
    board: &Board,
    depth: u32,
    _is_iterative: bool,
    timeout_ms: u64,
) -> (HyperMove, u32) {
    use std::sync::atomic::AtomicBool;
    let stop = AtomicBool::new(false);
    let mut searcher = TimedSearcher::new();
    let stats =
        searcher.search_with_stats(board, &SearchLimits::movetime(depth, timeout_ms), &stop);
    (stats.best_move, stats.completed_depth.max(1).min(depth))
}

fn strategic_with_timeout(board: &Board, depth: u32, timeout_ms: u64) -> (HyperMove, u32) {
    use std::sync::atomic::AtomicBool;
    let stop = AtomicBool::new(false);
    let mut searcher = TimedSearcher::strategic();
    let stats =
        searcher.search_with_stats(board, &SearchLimits::movetime(depth, timeout_ms), &stop);
    (stats.best_move, stats.completed_depth.max(1).min(depth))
}

fn pro_with_timeout(board: &Board, depth: u32, timeout_ms: u64) -> (HyperMove, u32) {
    use std::sync::atomic::AtomicBool;
    let stop = AtomicBool::new(false);
    let mut searcher = TimedSearcher::pro();
    let stats =
        searcher.search_with_stats(board, &SearchLimits::movetime(depth, timeout_ms), &stop);
    (stats.best_move, stats.completed_depth.max(1).min(depth))
}

/// MCTS search with automatic simulation halving on timeout.
/// Returns `(move, simulations_actually_used)`.
#[cfg(feature = "cuda")]
fn cuda_mcts_with_timeout(
    board: &Board,
    simulations: u32,
    threads: usize,
    batch_size: usize,
    timeout_ms: u64,
) -> (HyperMove, u32) {
    const MIN_SIMS: u32 = 100;
    let mut sims = simulations;
    loop {
        let b = board.clone();
        let s = sims;
        let t = threads;
        let bs = batch_size;
        match try_with_timeout(
            move || {
                if t > 1 {
                    cuda_mcts::mcts_cuda_parallel(&b, s, t, bs)
                } else {
                    cuda_mcts::mcts_cuda(&b, s, bs)
                }
            },
            timeout_ms,
        ) {
            Some(mv) if !mv.is_null() => return (mv, sims),
            _ => {
                let next = (sims / 2).max(MIN_SIMS);
                eprintln!(
                    "[TIMEOUT] MCTS {} sims exceeded {}ms — retrying at {} sims",
                    sims, timeout_ms, next
                );
                if sims <= MIN_SIMS {
                    // Absolute fallback: run synchronously at minimum budget.
                    let mv = cuda_mcts::mcts_cuda(board, MIN_SIMS, batch_size);
                    return (mv, MIN_SIMS);
                }
                sims = next;
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────

/// Returns `(best_move, backend_label, decision_rand)`.
/// `decision_rand` is a uniform f64 in [0, 1) drawn from the seeded RNG at the
/// exact moment the engine commits its chosen move — one value per ply,
/// ready for statistical analysis without any further normalisation.
///
/// `timeout_ms`: maximum milliseconds allowed per move.  On timeout the engine
/// retries with a weaker search (lower AB depth or fewer MCTS sims) until a
/// move is found.  0 = no timeout.
fn pick_move(
    board: &Board,
    gpu: &GpuBackend,
    cfg: &EngineConfig,
    rng: &mut SmallRng,
    timeout_ms: u64,
) -> (HyperMove, String, f64) {
    use rand::Rng;

    let name_lower = cfg.name.to_lowercase();
    let no_timeout = timeout_ms == 0;

    if name_lower == "cuda_mcts" {
        #[cfg(feature = "cuda")]
        {
            let (mv, sims_used) = if no_timeout {
                let mv = if cfg.threads > 1 {
                    cuda_mcts::mcts_cuda_parallel(
                        board,
                        cfg.simulations,
                        cfg.threads,
                        cfg.batch_size,
                    )
                } else {
                    cuda_mcts::mcts_cuda(board, cfg.simulations, cfg.batch_size)
                };
                (mv, cfg.simulations)
            } else {
                cuda_mcts_with_timeout(
                    board,
                    cfg.simulations,
                    cfg.threads,
                    cfg.batch_size,
                    timeout_ms,
                )
            };
            let dr: f64 = rng.gen();
            let label = if sims_used < cfg.simulations {
                format!("GPU-MCTS({}→{})", cfg.simulations, sims_used)
            } else {
                cfg.label(gpu)
            };
            return (mv, label, dr);
        }
        #[cfg(not(feature = "cuda"))]
        {
            let stop = std::sync::atomic::AtomicBool::new(false);
            let mv =
                hyperchess_search::mcts::mcts_bounded(board, cfg.simulations, timeout_ms, &stop);
            let dr: f64 = rng.gen();
            return (mv, format!("CPU-MCTS({})", cfg.simulations), dr);
        }
    }

    if name_lower == "mcts" {
        // Anytime MCTS: the wall-clock budget truncates the simulation count in
        // place of the old ignore-the-timeout behaviour.
        let stop = std::sync::atomic::AtomicBool::new(false);
        let mv = hyperchess_search::mcts::mcts_bounded(board, cfg.simulations, timeout_ms, &stop);
        let dr: f64 = rng.gen();
        return (mv, cfg.label(gpu), dr);
    }

    if name_lower == "random" {
        let moves = board.generate_moves();
        // For the random engine the draw itself selects the move — record it.
        let dr: f64 = rng.gen();
        let mv = if moves.is_empty() {
            HyperMove::null()
        } else {
            let idx = (dr * moves.len() as f64) as usize;
            *moves.iter().nth(idx.min(moves.len() - 1)).unwrap()
        };
        return (mv, "Random".to_string(), dr);
    }

    if name_lower == "strategic" || name_lower == "strategic" || name_lower == "strategic_like" {
        let (mv, depth_used) = if no_timeout {
            use hyperchess_rules::tools::Searcher;
            (
                TimedSearcher::strategic().best_move(board, cfg.depth),
                cfg.depth,
            )
        } else {
            strategic_with_timeout(board, cfg.depth, timeout_ms)
        };
        let dr: f64 = rng.gen();
        let label = if depth_used < cfg.depth {
            format!("CPU-Strategic(d{}→d{})", cfg.depth, depth_used)
        } else {
            format!("CPU-Strategic(d{})", cfg.depth)
        };
        return (mv, label, dr);
    }

    if name_lower == "aggressive" || name_lower == "commercial" || name_lower == "stockfish_like" {
        let (mv, depth_used) = if no_timeout {
            use hyperchess_rules::tools::Searcher;
            (TimedSearcher::pro().best_move(board, cfg.depth), cfg.depth)
        } else {
            pro_with_timeout(board, cfg.depth, timeout_ms)
        };
        let dr: f64 = rng.gen();
        let label = if depth_used < cfg.depth {
            format!("CPU-Aggressive(d{}→d{})", cfg.depth, depth_used)
        } else {
            format!("CPU-Aggressive(d{})", cfg.depth)
        };
        return (mv, label, dr);
    }

    // Compatibility guided alpha-beta labels. They use the canonical search.
    if name_lower == "guided" || name_lower == "guided_ab" {
        let mv = best_move_guided_parallel(board, cfg.depth, false);
        let dr: f64 = rng.gen();
        let label = format!("CPU-GAB(d{})", cfg.depth);
        return (mv, label, dr);
    }

    if name_lower == "guided_id" {
        let mv = best_move_guided_parallel(board, cfg.depth, true);
        let dr: f64 = rng.gen();
        let label = format!("CPU-GID(d{})", cfg.depth);
        return (mv, label, dr);
    }

    // Alpha-beta / iterative deepening — always run on CPU with Rayon parallelism.
    //
    // gpu_alphabeta_search launches a GPU batch-eval kernel at EVERY internal
    // node of the search tree.  At depth ≥ 5 on a 12×12 board the branching
    // factor (~50) makes this exponential: depth 6 → up to 50^3 = 125 000
    // blocking kernel calls per move.  CPU parallel AB with Rayon is bounded.
    let is_iterative = name_lower == "iterative" || name_lower == "id";
    let prefix = if is_iterative { "CPU-ID" } else { "CPU-AB" };

    let (mv, depth_used) = if no_timeout {
        (
            best_move_cpu_parallel(board, cfg.depth, is_iterative),
            cfg.depth,
        )
    } else {
        ab_with_timeout(board, cfg.depth, is_iterative, timeout_ms)
    };

    let dr: f64 = rng.gen();
    let label = if depth_used < cfg.depth {
        format!("{}(d{}→d{})", prefix, cfg.depth, depth_used)
    } else {
        format!("{}(d{})", prefix, cfg.depth)
    };
    (mv, label, dr)
}

/// Canonical CPU root search. All alpha-beta-family engine names ("alphabeta",
/// "iterative", "guided", "guided_id") now resolve to the single shared
/// [`TimedSearcher`], so the CLI gets exactly the same search — TT, killers/history
/// ordering, check extensions, LMR, null-move pruning, SEE-pruned quiescence — as
/// the web server and the browser WASM engine.
///
/// The old hand-rolled Rayon-over-root pattern is gone: one root search per move is
/// both stronger (deep iterative deepening with a shared TT) and avoids allocating a
/// transposition table per root move. CLI throughput comes from running whole games
/// in parallel (`--threads`), not from parallelising a single move.
fn best_move_cpu_parallel(board: &Board, depth: u32, _iterative: bool) -> HyperMove {
    use hyperchess_rules::tools::Searcher;
    TimedSearcher::new().best_move(board, depth)
}

/// Alias kept for the "guided" engine labels — same canonical search.
fn best_move_guided_parallel(board: &Board, depth: u32, _iterative: bool) -> HyperMove {
    use hyperchess_rules::tools::Searcher;
    TimedSearcher::new().best_move(board, depth)
}

// ── Game loop ─────────────────────────────────────────────────────────────────

fn export_base(out_dir: &str, timestamp: &str, game_seed: u64) -> String {
    format!("{}/{}_{:016x}", out_dir, timestamp, game_seed)
}

/// `verbose`: per-move/per-game commentary (board renders, one line per ply,
/// engine/seed banner). `false` for parallel bulk dataset generation — with
/// N games running concurrently (see `main.rs`'s `Commands::Play` handler),
/// interleaved per-move output from every game at once is both unreadable
/// and adds real stdout-lock contention across threads; the caller reports
/// aggregate progress instead. `true` preserves the exact original
/// single-game interactive experience. Export file writes and their
/// confirmation lines (from `export::save_*`) are unconditional either way
/// — knowing where output landed matters regardless of verbosity.
///
/// Inherited from the source repo otherwise verbatim (was already 10 params
/// before this addition — same treatment as GameStats::new in export.rs and
/// can_castle in hyperchess-rules, Phase 1).
#[allow(clippy::too_many_arguments)]
pub fn play_game(
    white: EngineConfig,
    black: EngineConfig,
    max_moves: u32,
    out_dir: &str,
    nnue_plain: bool,
    game_seed: u64,
    random_opening_plies: u32,
    white_skill: Option<u32>,
    black_skill: Option<u32>,
    move_timeout_ms: u64,
    verbose: bool,
) -> u32 {
    std::fs::create_dir_all(out_dir).ok();

    let gpu = detect_gpu();
    let effective_threads = {
        let t = white.threads.max(black.threads);
        if t == 0 {
            num_cpus::get()
        } else {
            t
        }
    };

    let gpu_name_opt: Option<String> = match &gpu {
        GpuBackend::Cuda(name) => Some(name.clone()),
        GpuBackend::None => None,
    };

    if verbose {
        println!("=== HyperChess Engine (CUDA build) ===");
        match &gpu {
            GpuBackend::Cuda(name) => println!("GPU      : {} — batch eval active", name),
            GpuBackend::None => println!(
                "GPU      : not detected — CPU ({} threads)",
                effective_threads
            ),
        }
        println!("Seed     : {}", game_seed);
    }

    // Safe to call once per concurrently-running game (see main.rs): every
    // call in a batch requests the identical `effective_threads` value
    // (white/black configs are shared across the whole --games run), so
    // whichever call actually wins the race to initialize rayon's global
    // pool sets the same size any other call would have. Only the first
    // call in the whole process actually takes effect; the rest are
    // harmless no-ops via `.ok()`, matching the pre-existing pattern.
    rayon::ThreadPoolBuilder::new()
        .num_threads(effective_threads)
        .build_global()
        .ok();

    let white_label = white.label(&gpu);
    let black_label = black.label(&gpu);

    let mut stats = GameStats::new(
        &white_label,
        &black_label,
        white.depth,
        black.depth,
        white.simulations,
        black.simulations,
        white_skill,
        black_skill,
        gpu_name_opt,
        game_seed,
    );

    if verbose {
        println!("White    : {}", white_label);
        println!("Black    : {}", black_label);
    }

    let mut board = Board::start_pos();
    if verbose {
        println!("{}", board);
    }

    // Seeded RNG: each move draws a snapshot for the log; random engine uses it directly.
    let mut rng = SmallRng::seed_from_u64(game_seed);

    let game_start = Instant::now();
    let mut move_count = 0u32;

    loop {
        if board.is_game_over() || move_count >= max_moves {
            break;
        }

        let is_white = board.turn() == Player::White;
        let side_str = if is_white { "White" } else { "Black" };
        let cfg = if is_white { &white } else { &black };

        let root_moves = board.generate_moves();
        let root_count = root_moves.len();
        let eval_before = eval::evaluate(&board);
        let t0 = Instant::now();

        // Opening plies are sampled uniformly from legal moves. Dataset jobs vary
        // this count across workers to avoid a single fixed opening horizon.
        let (best_move, backend, decision_rand) = if move_count < random_opening_plies {
            use rand::Rng;
            let dr: f64 = rng.gen();
            let moves = board.generate_moves();
            let mv = if moves.is_empty() {
                HyperMove::null()
            } else {
                let idx = (dr * moves.len() as f64) as usize;
                *moves.iter().nth(idx.min(moves.len() - 1)).unwrap()
            };
            (mv, "Random(opening)".to_string(), dr)
        } else {
            pick_move(&board, &gpu, cfg, &mut rng, move_timeout_ms)
        };
        let think_ms = t0.elapsed().as_millis();

        if best_move.is_null() {
            break;
        }

        let move_str = best_move.stringify();
        let hpgn_i =
            best_move.stringify_with_identity(board.piece_identity_at(best_move.get_src()));
        board.apply_move(best_move);
        let hfen_after = board.get_hfen();
        move_count += 1;

        if verbose {
            println!(
                "[{:>3}] {:5} {:8}  eval={:+5}cp  {:>4}ms  rand={:.6}  {}",
                move_count, side_str, move_str, eval_before, think_ms, decision_rand, backend
            );
        }

        stats.push_move(MoveRecord {
            ply: move_count,
            side: if is_white { "White" } else { "Black" },
            move_str,
            hpgn_i,
            hfen_after,
            eval_score: eval_before,
            think_ms,
            root_moves_count: root_count,
            backend,
            decision_rand,
        });
    }

    let total_game_ms = game_start.elapsed().as_millis();
    // Move-limit endings (external --max-moves cap or the internal no-progress
    // rule) are adjudicated by material: ≥ 1 pawn ahead wins, else draw.
    // HPGN-I/JSON exports match on "White wins"/"Black wins" substrings, which the
    // adjudicated strings preserve.
    let capped = move_count >= max_moves;
    let raw_result = board.game_result();
    let result = board.game_result_adjudicated(capped);
    let result_str = match (result, raw_result) {
        (1, 1) => "White wins",
        (2, 2) => "Black wins",
        (1, _) => "White wins by material adjudication (move limit)",
        (2, _) => "Black wins by material adjudication (move limit)",
        (3, 3) => match board.termination_reason() {
            "move_limit" => "Draw by move limit (material equal)",
            "fivefold_repetition" => "Draw by fivefold repetition",
            "threefold_repetition" => "Draw by threefold repetition",
            "insufficient_material" => "Draw by insufficient material",
            "stalemate" => "Draw by stalemate",
            _ => "Draw",
        },
        (3, _) => "Draw by move limit (material equal)",
        _ => "Ongoing",
    };

    stats.result = result_str.to_string();
    stats.total_moves = move_count;

    if verbose {
        println!("\n=== Game Over ===");
        println!(
            "Result   : {} | Plies: {} | Wall: {:.1}s",
            result_str,
            move_count,
            total_game_ms as f64 / 1000.0
        );
        println!("\nFinal position:\n{}", board);
    }

    let ts = chrono::Local::now().format("%Y%m%d_%H%M%S").to_string();
    let base = export_base(out_dir, &ts, game_seed);

    export::save_hfeni(&stats, &format!("{}.hfeni", base));
    export::save_game_stats(&board, &stats, &format!("{}.txt", base));
    export::save_json(&board, &stats, &format!("{}.json", base));
    export::save_hpgni(&stats, &format!("{}.hpgni", base));

    if nnue_plain {
        let plain_path = format!("{}/training.plain", out_dir);
        export::append_nnue_plain(&stats, &plain_path);
    }

    if verbose {
        println!("\nGame exported to {}_*", base);
    }
    move_count
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_export_base_is_unique_for_games_finishing_in_same_second() {
        let timestamp = "20260713_050000";
        let first = export_base("games/dataset_v1/raw", timestamp, 1001);
        let second = export_base("games/dataset_v1/raw", timestamp, 1002);

        assert_ne!(first, second);
        assert!(first.ends_with("_00000000000003e9"));
        assert!(second.ends_with("_00000000000003ea"));
    }

    #[test]
    fn test_parallel_search_does_not_corrupt_board() {
        hyperchess_rules::Helper::init();
        let board = Board::start_pos();
        let hfen_before = board.get_hfen();
        let mv = best_move_cpu_parallel(&board, 2, false);
        assert_eq!(board.get_hfen(), hfen_before);
        assert!(!mv.is_null());
    }

    #[test]
    fn test_compute_mastery_metrics() {
        use super::export::{compute_mastery, GameStats, MoveRecord};
        let mut stats = GameStats::new(
            "test_white",
            "test_black",
            4,
            4,
            100,
            100,
            Some(15),
            Some(15),
            None,
            1234,
        );
        stats.result = "White wins".to_string();
        stats.total_moves = 2;

        stats.push_move(MoveRecord {
            ply: 1,
            side: "White",
            move_str: "e2e4".to_string(),
            hpgn_i: "M:e2e4".to_string(),
            hfen_after: "12/abcdefghijkl/mnopqrstuvwx/12/12/12/12/12/12/MNOPQRSTUVWX/ABCDEFGHIJKL/12 w - - 0 1".to_string(),
            eval_score: 50,
            think_ms: 10,
            root_moves_count: 10,
            backend: "CPU".to_string(),
            decision_rand: 0.5,
        });

        stats.push_move(MoveRecord {
            ply: 2,
            side: "Black",
            move_str: "e7e5".to_string(),
            hpgn_i: "M:e7e5".to_string(),
            hfen_after: "12/abcdefghijkl/mnopqrstuvwx/12/12/12/12/12/12/MNOPQRSTUVWX/ABCDEFGHIJKL/12 w - - 0 1".to_string(),
            eval_score: -100,
            think_ms: 10,
            root_moves_count: 10,
            backend: "CPU".to_string(),
            decision_rand: 0.5,
        });

        let (w_m, b_m) = compute_mastery(&stats);
        assert!(w_m.soundness >= 0.0 && w_m.soundness <= 1.0);
        assert!(b_m.soundness >= 0.0 && b_m.soundness <= 1.0);
        assert!(w_m.mastery >= 0.0 && w_m.mastery <= 1.0);
        assert!(b_m.mastery >= 0.0 && b_m.mastery <= 1.0);
        assert!(w_m.v_target >= -1.0 && w_m.v_target <= 1.0);
        assert!(b_m.v_target >= -1.0 && b_m.v_target <= 1.0);
    }

    #[test]
    fn test_save_hfeni_and_hpgni() {
        use super::export::{save_hfeni, save_hpgni, GameStats, MoveRecord};
        let mut stats = GameStats::new(
            "test_white",
            "test_black",
            4,
            4,
            100,
            100,
            Some(15),
            Some(15),
            None,
            1234,
        );
        stats.result = "White wins".to_string();
        stats.total_moves = 1;

        stats.push_move(MoveRecord {
            ply: 1,
            side: "White",
            move_str: "e2e4".to_string(),
            hpgn_i: "M:e2e4".to_string(),
            hfen_after: "12/abcdefghijkl/mnopqrstuvwx/12/12/12/12/12/12/MNOPQRSTUVWX/ABCDEFGHIJKL/12 w - - 0 1".to_string(),
            eval_score: 50,
            think_ms: 10,
            root_moves_count: 10,
            backend: "CPU".to_string(),
            decision_rand: 0.5,
        });

        let hfeni_path = "./test_game.hfeni";
        let hpgni_path = "./test_game.hpgni";

        save_hfeni(&stats, hfeni_path);
        save_hpgni(&stats, hpgni_path);

        let hfeni_content = std::fs::read_to_string(hfeni_path).unwrap();
        let hpgni_content = std::fs::read_to_string(hpgni_path).unwrap();

        let lines: Vec<&str> = hfeni_content.lines().collect();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].contains("w - -"));

        assert!(hpgni_content.contains("[Event \"HyperChess CUDA Engine Selfplay\"]"));
        assert!(hpgni_content.contains("[White \"test_white\"]"));
        assert!(hpgni_content.contains("[Result \"1-0\"]"));
        assert!(hpgni_content.contains("1. M:e2e4 1-0"));

        std::fs::remove_file(hfeni_path).unwrap();
        std::fs::remove_file(hpgni_path).unwrap();
    }
}
