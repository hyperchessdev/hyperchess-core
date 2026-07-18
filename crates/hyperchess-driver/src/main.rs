//! `hyperchess` — the HyperChess driver binary.
//!
//! Subcommands: `play` / `perft` / `show` / `gpu-info` / `bench-eval` (CLI,
//! from `hyperchess_driver::cli`), `uci` (the native UCI server, from
//! `hyperchess_driver::uci`) — see docs/hyperchess-core-extraction-plan.md §12
//! Phase 4 for why these two previously-separate binaries
//! (`hyperchess`/`hyperchess-uci`) were consolidated into one with
//! subcommands (§13's "one binary, subcommands" recommendation) — and `api`
//! (the stateless REST/OpenAPI server, from `hyperchess_driver::api`, §12
//! Phase 5).

use clap::{Parser, Subcommand};
use hyperchess_driver::cli::game::EngineConfig;
use hyperchess_driver::{api, cli, uci};

const DEFAULT_OUT_DIR: &str = "./games";

#[derive(Parser)]
#[command(name = "hyperchess")]
#[command(about = "HyperChess engine driver — CLI + UCI. GPU-accelerated with --features cuda.")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

// `Play`'s ~20 clap-derive fields make it much larger than the other
// variants (clippy: 224 bytes total vs. 4 for the next-largest) — inherited
// from the source repo's exact same field set, not introduced here. Boxing
// `progress_file` to silence it would mean deref'ing it everywhere it's used
// below; not worth the churn for a clap arg struct that's constructed once
// per process and never hot-path-matched.
#[allow(clippy::large_enum_variant)]
#[derive(Subcommand)]
enum Commands {
    /// Play an engine-vs-engine game and export all results.
    ///
    /// Each side can use a different engine, depth, and simulation count,
    /// enabling direct comparisons (e.g. --white alphabeta --black cuda_mcts).
    Play {
        /// White engine: random | alphabeta | iterative | shredder | mcts | cuda_mcts
        #[arg(long, default_value = "cuda_mcts")]
        white: String,
        /// Black engine: random | alphabeta | iterative | shredder | mcts | cuda_mcts
        #[arg(long, default_value = "cuda_mcts")]
        black: String,

        /// Alpha-beta search depth for both sides (overridden by --white-depth / --black-depth)
        #[arg(long, default_value = "4")]
        depth: u32,
        /// Alpha-beta search depth for White (defaults to --depth)
        #[arg(long)]
        white_depth: Option<u32>,
        /// Alpha-beta search depth for Black (defaults to --depth)
        #[arg(long)]
        black_depth: Option<u32>,

        /// MCTS simulations per move for both sides (overridden by --white-simulations / --black-simulations)
        #[arg(long, default_value = "800")]
        simulations: u32,
        /// MCTS simulations per move for White (defaults to --simulations)
        #[arg(long)]
        white_simulations: Option<u32>,
        /// MCTS simulations per move for Black (defaults to --simulations)
        #[arg(long)]
        black_simulations: Option<u32>,

        /// GPU batch size for cuda_mcts (leaves evaluated per kernel launch)
        #[arg(long, default_value = "1024")]
        batch_size: usize,
        /// Maximum half-moves before declaring a draw by move limit (224 = 112 full moves)
        #[arg(long, default_value = "224")]
        max_moves: u32,
        /// Parallel threads (0 = auto-detect CPU cores)
        #[arg(long, default_value = "0")]
        threads: usize,
        /// Output directory for exported game files
        #[arg(long, default_value = DEFAULT_OUT_DIR)]
        out_dir: String,
        /// Number of games to play (>1 enables dataset generation mode)
        #[arg(long, default_value = "1")]
        games: u32,
        /// Output format: default | nnue-plain
        #[arg(long, default_value = "default")]
        format: String,
        /// RNG seed for reproducibility (0 = auto-generate from OS entropy)
        #[arg(long, default_value = "0")]
        random_seed: u64,
        /// Number of initial plies selected uniformly from legal moves.
        /// Use different values across dataset workers to diversify openings.
        #[arg(long, default_value = "2")]
        random_opening_plies: u32,
        /// Skill level for White (1–20). Informational only — stored in game logs.
        #[arg(long)]
        white_skill: Option<u32>,
        /// Skill level for Black (1–20). Informational only — stored in game logs.
        #[arg(long)]
        black_skill: Option<u32>,
        /// Per-move timeout in seconds. If a search exceeds this limit it is
        /// retried with a weaker configuration (lower AB depth or fewer MCTS
        /// simulations) until a move is found. 0 = no timeout (default).
        #[arg(long, default_value = "0")]
        move_timeout_secs: u64,
        /// Write a JSON progress snapshot here after each game (dataset-gen monitor UI).
        #[arg(long)]
        progress_file: Option<String>,
    },
    /// Run perft from starting position
    Perft {
        #[arg(default_value = "3")]
        depth: u32,
    },
    /// Print the starting position
    Show,
    /// Detect and report GPU/CUDA status
    GpuInfo,
    /// Benchmark GPU batch evaluation vs CPU (requires --features cuda)
    #[cfg(feature = "cuda")]
    BenchEval {
        #[arg(long, default_value = "1024")]
        n: usize,
    },
    /// Run the native UCI server (reads UCI commands from stdin, writes
    /// responses to stdout — usable by any UCI-capable GUI or client).
    Uci,
    /// Run the stateless REST/OpenAPI API server (binds HOST:PORT, default
    /// 0.0.0.0:8080 — see /docs for the Swagger UI once running).
    Api,
}

/// Build the atomic progress-file payload for a dataset-gen run. Pure — no I/O — so
/// it's directly unit-testable.
fn build_progress_payload(
    games_done: u32,
    games_total: u32,
    plies: &[u32],
    elapsed_secs: f64,
    white_engine: &str,
    black_engine: &str,
) -> serde_json::Value {
    let avg_ply = if plies.is_empty() {
        0.0
    } else {
        plies.iter().sum::<u32>() as f64 / plies.len() as f64
    };
    serde_json::json!({
        "games_done": games_done,
        "games_total": games_total,
        "avg_ply": avg_ply,
        "elapsed_secs": elapsed_secs,
        "white_engine": white_engine,
        "black_engine": black_engine,
    })
}

/// Atomic write, matching train.py's `_emit()` — write to `.tmp` then rename.
fn emit_progress(progress_file: &Option<String>, payload: &serde_json::Value) {
    let Some(path) = progress_file else { return };
    let tmp = format!("{path}.tmp");
    if std::fs::write(&tmp, payload.to_string()).is_ok() {
        let _ = std::fs::rename(&tmp, path);
    }
}

#[tokio::main]
async fn main() {
    hyperchess_rules::Helper::init();
    let parsed = Cli::parse();

    match parsed.command {
        Commands::Play {
            white,
            black,
            depth,
            white_depth,
            black_depth,
            simulations,
            white_simulations,
            black_simulations,
            batch_size,
            max_moves,
            threads,
            out_dir,
            games,
            format,
            random_seed,
            random_opening_plies,
            white_skill,
            black_skill,
            move_timeout_secs,
            progress_file,
        } => {
            let (derived_wd, derived_ws) = if let Some(s) = white_skill {
                (Some(s.div_ceil(2).clamp(1, 4)), Some((s * 100).max(100)))
            } else {
                (None, None)
            };
            let (derived_bd, derived_bs) = if let Some(s) = black_skill {
                (Some(s.div_ceil(2).clamp(1, 4)), Some((s * 100).max(100)))
            } else {
                (None, None)
            };

            let wd = white_depth.or(derived_wd).unwrap_or(depth);
            let bd = black_depth.or(derived_bd).unwrap_or(depth);
            let ws = white_simulations.or(derived_ws).unwrap_or(simulations);
            let bs = black_simulations.or(derived_bs).unwrap_or(simulations);
            let nnue_plain = format.to_lowercase() == "nnue-plain";
            let move_timeout_ms = move_timeout_secs * 1000;

            let white_cfg = EngineConfig::new(&white, wd, ws, batch_size, threads);
            let black_cfg = EngineConfig::new(&black, bd, bs, batch_size, threads);

            // Base seed: use provided value or generate from OS entropy.
            let base_seed: u64 = if random_seed == 0 {
                use rand::RngCore;
                rand::thread_rng().next_u64()
            } else {
                random_seed
            };

            let run_start = std::time::Instant::now();

            if games > 1 {
                // Bulk dataset generation: run games concurrently, one OS
                // thread per game, up to --threads (0 = all cores). Safe
                // because (a) each game writes to its own uniquely-seeded
                // export_base()-derived file set — no cross-game path
                // collisions — and (b) append_nnue_plain's shared
                // training.plain writer buffers a whole game into memory
                // first and does exactly one OpenOptions(append=true)
                // write_all() call — POSIX O_APPEND makes that one syscall
                // atomic across concurrent writers, verified by reading
                // export.rs's implementation before relying on it, not
                // assumed. No searcher in hyperchess-search uses rayon
                // internally (verified in Phase 3), so this is the only
                // real lever for dataset-gen throughput — N independent
                // single-threaded games in parallel, not one game trying to
                // parallelize a search that has nothing to parallelize.
                use rayon::prelude::*;
                use std::sync::atomic::{AtomicU32, Ordering};
                use std::sync::Mutex;

                let effective_threads = if threads == 0 {
                    num_cpus::get()
                } else {
                    threads
                };
                rayon::ThreadPoolBuilder::new()
                    .num_threads(effective_threads)
                    .build_global()
                    .ok();

                println!(
                    "Generating {games} games across up to {effective_threads} parallel workers (white={white} black={black})..."
                );

                let plies_so_far: Mutex<Vec<u32>> = Mutex::new(Vec::with_capacity(games as usize));
                let games_done = AtomicU32::new(0);

                (1..=games).into_par_iter().for_each(|g| {
                    let game_seed = base_seed.wrapping_add(g as u64);
                    let plies = cli::game::play_game(
                        white_cfg.clone(),
                        black_cfg.clone(),
                        max_moves,
                        &out_dir,
                        nnue_plain,
                        game_seed,
                        random_opening_plies,
                        white_skill,
                        black_skill,
                        move_timeout_ms,
                        false, // verbose — see play_game's doc comment
                    );

                    let done = games_done.fetch_add(1, Ordering::Relaxed) + 1;
                    let snapshot = {
                        let mut guard = plies_so_far.lock().unwrap();
                        guard.push(plies);
                        guard.clone()
                    };
                    emit_progress(
                        &progress_file,
                        &build_progress_payload(
                            done,
                            games,
                            &snapshot,
                            run_start.elapsed().as_secs_f64(),
                            &white,
                            &black,
                        ),
                    );
                    let elapsed = run_start.elapsed().as_secs_f64();
                    if done.is_multiple_of(10) || done == games {
                        println!(
                            "[{done:>6}/{games}] {:.1}s elapsed, {:.2} games/s",
                            elapsed,
                            done as f64 / elapsed.max(0.001)
                        );
                    }
                });

                let elapsed = run_start.elapsed().as_secs_f64();
                println!(
                    "\nDone: {games} games in {elapsed:.1}s ({:.2} games/s) → {out_dir}",
                    games as f64 / elapsed.max(0.001)
                );
                if nnue_plain {
                    println!("NNUE training data → {out_dir}/training.plain");
                }
            } else {
                // Single game: unchanged interactive behavior (verbose output).
                let game_seed = base_seed.wrapping_add(1);
                let plies = cli::game::play_game(
                    white_cfg,
                    black_cfg,
                    max_moves,
                    &out_dir,
                    nnue_plain,
                    game_seed,
                    random_opening_plies,
                    white_skill,
                    black_skill,
                    move_timeout_ms,
                    true, // verbose
                );
                emit_progress(
                    &progress_file,
                    &build_progress_payload(
                        1,
                        1,
                        &[plies],
                        run_start.elapsed().as_secs_f64(),
                        &white,
                        &black,
                    ),
                );
            }
        }

        Commands::Perft { depth } => {
            let mut board = hyperchess_rules::Board::start_pos();
            println!("Running perft({})...", depth);
            let nodes = hyperchess_rules::board::perft::perft(&mut board, depth);
            println!("Perft({}) = {}", depth, nodes);
        }

        Commands::Show => {
            println!("{}", hyperchess_rules::Board::start_pos());
        }

        Commands::GpuInfo => match cli::game::detect_gpu() {
            cli::game::GpuBackend::Cuda(name) => {
                println!("CUDA available: {}", name);
                #[cfg(feature = "cuda")]
                {
                    if let Ok(d) = cust::device::Device::get_device(0) {
                        if let (Ok(maj), Ok(min)) = (
                            d.get_attribute(cust::device::DeviceAttribute::ComputeCapabilityMajor),
                            d.get_attribute(cust::device::DeviceAttribute::ComputeCapabilityMinor),
                        ) {
                            println!("  Compute capability : {}.{}", maj, min);
                        }
                        if let Ok(mem) = d.total_memory() {
                            println!("  Total VRAM : {:.1} GiB", mem as f64 / (1u64 << 30) as f64);
                        }
                    }
                }
                println!("  GPU engines: cuda_mcts, alphabeta (gpu_root_search)");
            }
            cli::game::GpuBackend::None => {
                println!("CUDA not detected. CPU cores: {}", num_cpus::get());
            }
        },

        #[cfg(feature = "cuda")]
        Commands::BenchEval { n } => {
            bench_eval(n);
        }

        Commands::Uci => uci::run(),

        Commands::Api => {
            if let Err(e) = api::run().await {
                eprintln!("api server error: {e}");
                std::process::exit(1);
            }
        }
    }
}

#[cfg(feature = "cuda")]
fn bench_eval(n: usize) {
    use hyperchess_rules::tools::eval::evaluate_base as evaluate;
    use hyperchess_search_cuda::cuda_backend;
    use rand::rngs::SmallRng;
    use rand::{Rng, SeedableRng};

    let n = n.max(1);
    println!("Benchmark + cross-validation: {} diverse positions", n);

    // Build n diverse positions by walking pseudo-random games (deterministic seed).
    let mut rng = SmallRng::seed_from_u64(0xC0FFEE_1234_5678);
    let mut boards: Vec<hyperchess_rules::Board> = Vec::with_capacity(n);
    let mut b = hyperchess_rules::Board::start_pos();
    while boards.len() < n {
        boards.push(b.clone());
        let moves = b.generate_moves();
        if moves.is_empty() || b.is_game_over() {
            b = hyperchess_rules::Board::start_pos();
            continue;
        }
        let idx = rng.gen_range(0..moves.len());
        b.apply_move(moves.get(idx));
    }

    // CPU reference scores.
    let t_cpu = std::time::Instant::now();
    let cpu_scores: Vec<i32> = boards.iter().map(|b| evaluate(b)).collect();
    let cpu_ms = t_cpu.elapsed().as_secs_f64() * 1000.0;

    // Warmup: first GPU call pays one-time CUDA context init + PTX JIT; exclude it.
    if let Err(e) = cuda_backend::gpu_batch_eval(&boards[..1]) {
        eprintln!("GPU batch eval failed: {e}");
        return;
    }

    let t_gpu = std::time::Instant::now();
    match cuda_backend::gpu_batch_eval(&boards) {
        Ok(gpu_scores) => {
            let gpu_ms = t_gpu.elapsed().as_secs_f64() * 1000.0;

            // Correctness: GPU must equal CPU on every position.
            let mut mismatches = 0usize;
            for (i, (c, g)) in cpu_scores.iter().zip(gpu_scores.iter()).enumerate() {
                if c != g {
                    if mismatches < 5 {
                        let stm = if boards[i].turn() == hyperchess_rules::Player::White {
                            'w'
                        } else {
                            'b'
                        };
                        println!(
                            "  MISMATCH[{i}] CPU={c} GPU={g} (stm={stm}) hfen={}",
                            boards[i].get_hfen()
                        );
                    }
                    mismatches += 1;
                }
            }
            println!(
                "Cross-validation: {}/{} match{}",
                n - mismatches,
                n,
                if mismatches == 0 {
                    "  ✓ CPU == GPU"
                } else {
                    "  ✗ DIVERGENCE"
                }
            );

            println!("CPU : {:.2}ms  ({:.0} kpos/s)", cpu_ms, n as f64 / cpu_ms);
            println!(
                "GPU : {:.2}ms  ({:.0} kpos/s)  [warmed, JIT excluded]",
                gpu_ms,
                n as f64 / gpu_ms
            );
            println!("Speedup: {:.2}x", cpu_ms / gpu_ms.max(0.001));
        }
        Err(e) => eprintln!("GPU batch eval failed: {e}"),
    }
}

#[cfg(test)]
mod progress_tests {
    use super::*;

    #[test]
    fn progress_payload_shape() {
        let payload = build_progress_payload(3, 10, &[42, 58, 51], 12.5, "cuda_mcts", "shredder");
        assert_eq!(payload["games_done"], 3);
        assert_eq!(payload["games_total"], 10);
        assert_eq!(payload["avg_ply"], (42 + 58 + 51) as f64 / 3.0);
        assert_eq!(payload["white_engine"], "cuda_mcts");
        assert_eq!(payload["black_engine"], "shredder");
    }
}
