//! One-off measurement harness to capture golden regression values.
//! Run: cargo run -p hyperchess-search --release --example golden_measure
//! The printed values are baked into tests/regression.rs as assertions.

use std::time::Instant;

use hyperchess_rules::board::perft::perft;
use hyperchess_rules::tools::eval::evaluate;
use hyperchess_rules::tools::Searcher;
use hyperchess_rules::Board;
use hyperchess_search::{AlphaBetaSearcher, GuidedAlphaBeta, GuidedIterative, IterativeSearcher};

/// Advance a position by applying the first generated move `n` times.
/// Movegen order is deterministic, so this yields reproducible positions
/// independent of any searcher.
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

fn main() {
    println!("=== PERFT (start) ===");
    for depth in 1..=4 {
        let mut b = Board::start_pos();
        let t = Instant::now();
        let nodes = perft(&mut b, depth);
        println!("perft({depth}) = {nodes}   [{:?}]", t.elapsed());
    }

    let positions = [("P0_start", walk(0)), ("P4", walk(4)), ("P8", walk(8))];

    println!("\n=== POSITION FENS + perft 1..3 ===");
    for (name, b) in &positions {
        println!("{name} HFEN = {}", b.get_hfen());
        for depth in 1..=3 {
            let mut bb = b.clone();
            println!("  perft({depth}) = {}", perft(&mut bb, depth));
        }
    }

    println!("\n=== SEARCHER best_move @ depth 3 (deterministic searchers) ===");
    for (name, b) in &positions {
        let ab = AlphaBetaSearcher::new().best_move(b, 3).stringify();
        let id = IterativeSearcher::new().best_move(b, 3).stringify();
        let gab = GuidedAlphaBeta::new().best_move(b, 3).stringify();
        let gid = GuidedIterative::new().best_move(b, 3).stringify();
        println!("{name}: AB={ab} ID={id} GAB={gab} GID={gid}");
    }

    println!("\n=== EVAL (side-to-move perspective) ===");
    for (name, b) in &positions {
        println!("{name}: eval = {}", evaluate(b));
    }
}
