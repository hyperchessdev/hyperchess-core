// SPDX-License-Identifier: GPL-3.0-or-later
// HyperChess Core — hyperchess-rules
// File: crates/hyperchess-rules/tests/test_simulator.rs
// Version: 1.0.0
// Copyright (c) 2026 HyperChess Developer Team

//! Simulator and Game Judge verification tests. (Phase 02)

use hyperchess_rules::Board;
use rand::prelude::IteratorRandom;
use rand::thread_rng;

#[test]
fn test_1000_random_games() {
    let mut rng = thread_rng();
    let mut draws = 0;
    let mut white_wins = 0;
    let mut black_wins = 0;

    for _ in 0..1000 {
        let mut board = Board::start_pos();
        let mut move_count = 0;

        while !board.is_game_over() && move_count < 1000 {
            let moves = board.generate_moves();
            if moves.is_empty() {
                break;
            }

            // Choose a random move
            let &m = moves.iter().choose(&mut rng).unwrap();
            board.apply_move(m);
            move_count += 1;
        }

        // Assert game result correctness
        let result = board.game_result();
        match result {
            1 => white_wins += 1,
            2 => black_wins += 1,
            3 => draws += 1,
            0 => {
                // If it ended due to move limit, we don't count it as finished.
                assert_eq!(move_count, 1000);
            }
            _ => panic!("Invalid game result: {}", result),
        }
    }

    println!(
        "Random games stats — White Wins: {}, Black Wins: {}, Draws: {}",
        white_wins, black_wins, draws
    );
}
