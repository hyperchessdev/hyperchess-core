// SPDX-License-Identifier: GPL-3.0-or-later
// HyperChess Core — hyperchess-search
// File: crates/hyperchess-search/examples/node_cap_probe.rs
// Version: 1.0.0
// Copyright (c) 2026 HyperChess Developer Team

//! Probe: what search depth does a given node budget actually buy?
//!
//! The web WASM worker (engine-worker.js `best_move_strategic`) runs strategic/pro
//! searches with `nodeLimit ?? 1_500_000` and no wall clock. This example runs
//! the same `TimedSearcher` on positions read from stdin (one HFEN per line) at a
//! requested depth ceiling and node budget, and reports the *completed* depth —
//! i.e. the effective skill after the cap.
//!
//! Usage: cargo run --release -p hyperchess-search --example node_cap_probe -- <max_depth> <node_limit> < fens.txt

use std::io::BufRead;
use std::sync::atomic::AtomicBool;

use hyperchess_rules::board::Board;
use hyperchess_search::{SearchLimits, SearchProfile, TimedSearcher};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let max_depth: u32 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(6);
    let node_limit: u64 = args
        .get(2)
        .and_then(|s| s.parse().ok())
        .unwrap_or(1_500_000);

    hyperchess_rules::Helper::init();
    println!("depth ceiling {max_depth}, node budget {node_limit}");
    println!(
        "{:<10} {:>10} {:>15} {:>8}",
        "completed", "nodes", "ms", "aborted"
    );

    let stdin = std::io::stdin();
    for line in stdin.lock().lines() {
        let hfen = match line {
            Ok(l) if !l.trim().is_empty() => l,
            _ => continue,
        };
        let board = match Board::from_hfen(hfen.trim()) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("bad hfen ({e:?}): {hfen}");
                continue;
            }
        };
        let stop = AtomicBool::new(false);
        let mut searcher = TimedSearcher::with_profile(SearchProfile::Strategic);
        let t0 = std::time::Instant::now();
        let stats =
            searcher.search_with_stats(&board, &SearchLimits::nodes(max_depth, node_limit), &stop);
        println!(
            "{:<10} {:>10} {:>15.0} {:>8}",
            stats.completed_depth,
            stats.nodes,
            t0.elapsed().as_millis(),
            stats.aborted
        );
    }
}
