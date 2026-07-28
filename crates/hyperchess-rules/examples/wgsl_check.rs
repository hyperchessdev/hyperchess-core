// SPDX-License-Identifier: GPL-3.0-or-later
// HyperChess Core — hyperchess-rules
// File: crates/hyperchess-rules/examples/wgsl_check.rs
// Version: 1.0.0
// Copyright (c) 2026 HyperChess Developer Team

//! Dumps `(encoding, side_to_move, engine_eval)` for a set of positions as JSON
//! lines, so the WGSL shader's evaluation (ported to JS in tools/wgsl_check.mjs)
//! can be cross-validated against the authoritative engine evaluator.
//!
//! Run: cargo run -p hyperchess --release --example wgsl_check

use hyperchess_rules::core::sq::SQ;
use hyperchess_rules::tools::eval::evaluate;
use hyperchess_rules::{Board, Player};

fn encode(board: &Board) -> Vec<u8> {
    (0u8..144).map(|i| board.piece_at(SQ(i)) as u8).collect()
}

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
    // A spread of positions: start, first-move walks (varied material/king pos).
    let boards: Vec<Board> = (0..40).map(walk).collect();
    for b in &boards {
        let enc = encode(b);
        let side = if b.turn() == Player::White { 0 } else { 1 };
        let eval = evaluate(b);
        let enc_str = enc
            .iter()
            .map(|x| x.to_string())
            .collect::<Vec<_>>()
            .join(",");
        println!(
            "{{\"side\":{},\"eval\":{},\"enc\":[{}]}}",
            side, eval, enc_str
        );
    }
}
