//! Native HyperChess UCI server.
//!
//! Reads UCI commands from stdin, runs the native Rust engine, and writes
//! UCI responses to stdout.  This binary is `hyperchess-uci` and can be
//! used anywhere a UCI-capable engine is expected.
//!
//! Supported commands:
//!   uci, isready, ucinewgame, position [fen <fen> | startpos] [moves m1 m2 …],
//!   go [depth N | movetime N | infinite], stop, quit, d (debug display)
//!
//! Unsupported (silently ignored): setoption, register, ponderhit

use std::io::{self, BufRead, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Instant;

use hyperchess_rules::board::perft::perft;
use hyperchess_rules::tools::eval;
use hyperchess_rules::{Board, Player};
use hyperchess_search::{SearchLimits, TimedSearcher};

use super::server_util::{parse_fen_and_moves, parse_go};

/// Run the UCI server loop, reading from `stdin` and writing to `stdout`.
pub fn run() {
    hyperchess_rules::Helper::init();

    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut out = stdout.lock();

    // Shared board state — the position command updates this.
    let board: Arc<Mutex<Board>> = Arc::new(Mutex::new(Board::start_pos()));
    // Flag to request search stop. An `AtomicBool` (not a mutex) because it is
    // passed into `TimedSearcher::search`, which polls it *inside* the recursive
    // search — `stop` therefore interrupts a depth in progress, not just the
    // gap between depths.
    let stop_flag: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let tokens: Vec<&str> = line.split_whitespace().collect();
        let cmd = tokens[0];

        match cmd {
            "uci" => {
                writeln!(out, "id name HyperChess Native Engine").ok();
                writeln!(out, "id author HyperChess Team").ok();
                writeln!(out, "option name Hash type spin default 64 min 1 max 8192").ok();
                writeln!(out, "option name Threads type spin default 1 min 1 max 64").ok();
                writeln!(out, "uciok").ok();
            }

            "isready" => {
                writeln!(out, "readyok").ok();
            }

            "ucinewgame" => {
                *board.lock().unwrap() = Board::start_pos();
                stop_flag.store(false, Ordering::Relaxed);
            }

            "position" => {
                if let Some(new_board) = parse_fen_and_moves(&tokens[1..]) {
                    *board.lock().unwrap() = new_board;
                }
            }

            "go" => {
                stop_flag.store(false, Ordering::Relaxed);
                let go_args = parse_go(&tokens[1..]);
                let board_clone = Arc::clone(&board);
                let stop_clone = Arc::clone(&stop_flag);
                let mut out2 = io::stdout();

                thread::spawn(move || {
                    search_and_respond(board_clone, stop_clone, go_args, &mut out2);
                });
            }

            "stop" => {
                stop_flag.store(true, Ordering::Relaxed);
            }

            "d" => {
                writeln!(out, "{}", board.lock().unwrap()).ok();
                writeln!(out, "Fen: {}", board.lock().unwrap().get_hfen()).ok();
            }

            "perft" => {
                let depth: u32 = tokens.get(1).and_then(|s| s.parse().ok()).unwrap_or(1);
                let mut b = board.lock().unwrap().clone();
                drop(b.clone());
                let start = Instant::now();
                let nodes = perft(&mut b, depth);
                let elapsed = start.elapsed().as_millis().max(1);
                writeln!(
                    out,
                    "Nodes searched: {nodes}  ({} kN/s)",
                    nodes as u128 / elapsed
                )
                .ok();
            }

            "quit" | "exit" => break,

            // Silently ignore unknown / unimplemented commands.
            _ => {}
        }

        out.flush().ok();
    }
}

// ── Search ────────────────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct GoArgs {
    pub depth: u32,
    pub movetime_ms: u64,
    pub infinite: bool,
    pub perft_depth: Option<u32>,
}

impl Default for GoArgs {
    fn default() -> Self {
        Self {
            depth: 5,
            movetime_ms: 0,
            infinite: false,
            perft_depth: None,
        }
    }
}

fn search_and_respond(
    board: Arc<Mutex<Board>>,
    stop_flag: Arc<AtomicBool>,
    args: GoArgs,
    out: &mut impl Write,
) {
    if let Some(pd) = args.perft_depth {
        let mut b = board.lock().unwrap().clone();
        let nodes = perft_divide_to_writer(&mut b, pd, out);
        writeln!(out, "\nNodes searched: {nodes}").ok();
        out.flush().ok();
        return;
    }

    let b = board.lock().unwrap().clone();
    let moves = b.generate_moves();

    if moves.is_empty() {
        writeln!(out, "bestmove 0000").ok();
        out.flush().ok();
        return;
    }

    let start = Instant::now();
    let max_depth = if args.infinite { 64 } else { args.depth.max(1) };

    let mut best_move = *moves.iter().next().unwrap();

    // Iterative deepening. The stop flag and the remaining movetime budget are
    // passed *into* every `search_with_stats` call, so `stop`/`go movetime` can
    // interrupt a depth in progress — the anytime search returns the best move
    // from the deepest completed depth the moment either triggers.
    let mut searcher = TimedSearcher::new();
    for d in 1..=max_depth {
        if stop_flag.load(Ordering::Relaxed) {
            break;
        }
        let remaining_ms = if args.movetime_ms > 0 {
            let remaining = args
                .movetime_ms
                .saturating_sub(start.elapsed().as_millis() as u64);
            if remaining == 0 {
                break;
            }
            remaining
        } else {
            0 // no clock
        };

        let limits = SearchLimits {
            max_depth: d,
            node_limit: 0,
            movetime_ms: remaining_ms,
        };
        let stats = searcher.search_with_stats(&b, &limits, &stop_flag);
        if !stats.best_move.is_null() {
            best_move = stats.best_move;
        }
        if stats.aborted {
            break;
        }

        // Evaluate the position after the best move for the score.
        let mut b_after = b.clone();
        b_after.apply_move(best_move);
        let raw_score = -eval::evaluate(&b_after); // negated: eval is from STM perspective
        let side_sign = if b.turn() == Player::White { 1 } else { -1 };
        let score_cp = raw_score * side_sign;

        writeln!(out, "info depth {d} score cp {score_cp} pv {best_move}",).ok();
        out.flush().ok();
    }

    writeln!(out, "bestmove {best_move}").ok();
    out.flush().ok();
}

fn perft_divide_to_writer(board: &mut Board, depth: u32, out: &mut impl Write) -> u64 {
    let moves = board.generate_moves();
    let mut total = 0u64;
    for m in moves.iter() {
        board.apply_move(*m);
        let count = if depth <= 1 {
            1
        } else {
            perft(board, depth - 1)
        };
        total += count;
        writeln!(out, "{m}: {count}").ok();
        board.undo_move();
    }
    total
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn respond(args: GoArgs, stop: Arc<AtomicBool>) -> String {
        let board = Arc::new(Mutex::new(Board::start_pos()));
        let mut out: Vec<u8> = Vec::new();
        search_and_respond(board, stop, args, &mut out);
        String::from_utf8(out).expect("utf8 UCI output")
    }

    /// `stop` flipped from another thread must interrupt a deep search *within*
    /// a depth, not after it — the historical bug was a per-call local stop flag
    /// that let `go depth 64` run a full depth past the stop command.
    #[test]
    fn stop_interrupts_a_deep_search_within_a_depth() {
        hyperchess_rules::Helper::init();
        let stop = Arc::new(AtomicBool::new(false));
        let stopper = Arc::clone(&stop);
        let handle = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(200));
            stopper.store(true, Ordering::Relaxed);
        });

        let start = Instant::now();
        let output = respond(
            GoArgs {
                depth: 64,
                movetime_ms: 0,
                infinite: true,
                perft_depth: None,
            },
            stop,
        );
        let elapsed = start.elapsed();
        handle.join().unwrap();

        // Depth 64 from the start position takes far longer than this without
        // in-depth stop propagation. Generous bound to stay CI-safe.
        assert!(
            elapsed < Duration::from_secs(10),
            "stop did not interrupt the search (took {elapsed:?})"
        );
        assert!(output.contains("bestmove "), "missing bestmove: {output}");
        assert!(!output.contains("bestmove 0000"), "null bestmove: {output}");
    }

    /// `go movetime N` must bound the whole search even when a single depth
    /// would exceed the budget on its own.
    #[test]
    fn movetime_bounds_the_search_within_a_depth() {
        hyperchess_rules::Helper::init();
        let start = Instant::now();
        let output = respond(
            GoArgs {
                depth: 64,
                movetime_ms: 300,
                infinite: false,
                perft_depth: None,
            },
            Arc::new(AtomicBool::new(false)),
        );
        let elapsed = start.elapsed();

        assert!(
            elapsed < Duration::from_secs(10),
            "movetime did not bound the search (took {elapsed:?})"
        );
        assert!(output.contains("bestmove "), "missing bestmove: {output}");
        assert!(!output.contains("bestmove 0000"), "null bestmove: {output}");
    }
}
