// SPDX-License-Identifier: GPL-3.0-or-later
// HyperChess Core — hyperchess-rules
// File: crates/hyperchess-rules/src/board/display.rs
// Version: 1.0.0
// Copyright (c) 2026 HyperChess Developer Team

//! Human-readable board rendering.

use super::Board;
use crate::core::sq::SQ;
use crate::core::Piece;

impl Board {
    /// Print the board to a string for debugging.
    pub fn pretty_print(&self) -> String {
        let mut s = String::new();
        for rank in (0..12u8).rev() {
            s.push_str(&format!("{:>2} ", rank + 1));
            for file in 0..12u8 {
                let sq = SQ::make(file, rank);
                let piece = self.piece_at(sq);
                if piece == Piece::None {
                    s.push_str(". ");
                } else {
                    s.push(
                        self.piece_hfen_char_at(sq)
                            .unwrap_or_else(|| piece.character_lossy()),
                    );
                    s.push(' ');
                }
            }
            s.push('\n');
        }
        s.push_str("   a b c d e f g h i j k l\n");
        s.push_str(&format!(
            "Turn: {} | HFEN: {}\n",
            self.turn(),
            self.get_hfen()
        ));
        s
    }
}

impl std::fmt::Display for Board {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.pretty_print())
    }
}
