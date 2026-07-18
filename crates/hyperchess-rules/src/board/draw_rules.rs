//! Game-termination and draw rules: checkmate/stalemate, the move rule,
//! repetition and insufficient material — plus material adjudication for
//! move-limit endings.

use super::Board;
use crate::core::score::{Value, PIECE_VALUE_EG};
use crate::core::{PieceType, Player};

/// Material edge (centipawns) required to award a move-limit adjudication win.
/// One pawn: any clear material advantage decides; less than a pawn is a draw.
pub const MATERIAL_ADJUDICATION_MARGIN: Value = 100;

impl Board {
    /// Returns true if the game is over (checkmate, stalemate, or draw by rule).
    pub fn is_game_over(&self) -> bool {
        let moves = self.generate_moves();
        if moves.is_empty() {
            return true; // Checkmate or stalemate
        }
        // Move rule: scaled from 50 (8×8) to 112 full moves (12×12, area ratio ≈ 2.25×)
        if self.state.rule50 >= 224 {
            return true;
        }
        // Fivefold repetition (absolute draw rule): 4 prior occurrences + current = 5 total.
        if self.repetition_count() >= 4 {
            return true;
        }
        // Threefold repetition (claimable draw — treat as forced draw here):
        // 2 prior occurrences + current = 3 total.
        if self.repetition_count() >= 2 {
            return true;
        }
        // Insufficient material
        if self.insufficient_material() {
            return true;
        }
        false
    }

    /// Returns why the game ended (or "ongoing"): "checkmate", "stalemate",
    /// "move_limit" (112-full-move/224-halfmove no-progress rule), "fivefold_repetition",
    /// "threefold_repetition", or "insufficient_material". Single source of truth for
    /// this precedence — callers (WASM bindings, the server termination endpoint, the
    /// CLI) should use this instead of re-deriving it, so they never drift out of sync.
    pub fn termination_reason(&self) -> &'static str {
        let moves = self.generate_moves();
        if moves.is_empty() {
            return if self.in_check() {
                "checkmate"
            } else {
                "stalemate"
            };
        }
        if self.state.rule50 >= 224 {
            return "move_limit";
        }
        if self.repetition_count() >= 4 {
            return "fivefold_repetition";
        }
        if self.repetition_count() >= 2 {
            return "threefold_repetition";
        }
        if self.insufficient_material() {
            return "insufficient_material";
        }
        "ongoing"
    }

    /// Returns the game result: 0=ongoing, 1=white wins, 2=black wins, 3=draw.
    pub fn game_result(&self) -> u8 {
        let moves = self.generate_moves();
        if moves.is_empty() {
            if self.in_check() {
                // Checkmate: the opponent (who just moved) wins
                return if self.turn() == Player::White { 2 } else { 1 };
            } else {
                return 3; // Stalemate
            }
        }
        if self.state.rule50 >= 224 {
            return 3;
        } // 112-move rule
        if self.repetition_count() >= 2 {
            return 3;
        } // threefold/fivefold repetition (2 prior + current = 3 total)
        if self.insufficient_material() {
            return 3;
        } // insufficient material
        0 // Ongoing
    }

    /// Count how many times the current position occurred **before** now.
    ///
    /// Contract: `Board.history` stores *prior* positions only — `apply_move`
    /// pushes the pre-move hash, and the current position is never in it. So a
    /// return of 2 means "2 prior occurrences + the current board = threefold".
    /// Callers importing external history (WASM/server `fen_history`) must keep
    /// that contract: never push the current position.
    pub fn repetition_count(&self) -> usize {
        let cur = self.state.zobrist;
        self.history.iter().filter(|&&z| z == cur).count()
    }

    /// True if neither side has sufficient material to force checkmate.
    pub fn insufficient_material(&self) -> bool {
        // Need at least one pawn, rook, queen, eagle, or hawk to force mate
        let heavy = [
            PieceType::P,
            PieceType::R,
            PieceType::Q,
            PieceType::E,
            PieceType::H,
        ];
        for &pt in &heavy {
            for &pl in &[Player::White, Player::Black] {
                if !self.piece_bb(pl, pt).is_empty() {
                    return false;
                }
            }
        }
        // Count minor pieces (knights + bishops) per side
        let wn = self.piece_bb(Player::White, PieceType::N).count_bits() as usize;
        let wb = self.piece_bb(Player::White, PieceType::B).count_bits() as usize;
        let bn = self.piece_bb(Player::Black, PieceType::N).count_bits() as usize;
        let bb = self.piece_bb(Player::Black, PieceType::B).count_bits() as usize;
        // KvK, KNvK, KBvK, KNvKN, KBvKB (same colour) → insufficient
        (wn + wb) <= 1 && (bn + bb) <= 1
    }

    /// Raw material balance in centipawns, White minus Black (kings excluded).
    pub fn material_diff(&self) -> Value {
        let pieces = [
            PieceType::P,
            PieceType::N,
            PieceType::B,
            PieceType::R,
            PieceType::Q,
            PieceType::E,
            PieceType::H,
        ];
        let mut diff: Value = 0;
        for &pt in &pieces {
            let v = PIECE_VALUE_EG[pt as usize];
            diff += v * self.piece_bb(Player::White, pt).count_bits() as Value;
            diff -= v * self.piece_bb(Player::Black, pt).count_bits() as Value;
        }
        diff
    }

    /// Adjudicate a position that ended without a decisive result (move-limit):
    /// 1 = White wins, 2 = Black wins, 3 = draw. The side with at least
    /// [`MATERIAL_ADJUDICATION_MARGIN`] more material wins; otherwise a draw.
    pub fn adjudicate_material(&self) -> u8 {
        let diff = self.material_diff();
        if diff >= MATERIAL_ADJUDICATION_MARGIN {
            1
        } else if diff <= -MATERIAL_ADJUDICATION_MARGIN {
            2
        } else {
            3
        }
    }

    /// [`Self::game_result`] with material adjudication for move-limit endings.
    ///
    /// * Decisive results and genuine draws (stalemate, repetition, insufficient
    ///   material) are returned unchanged.
    /// * A draw by the internal no-progress rule (`termination_reason() ==
    ///   "move_limit"`) is adjudicated by material.
    /// * `capped` = the game hit an *external* per-game move cap (e.g. the web
    ///   lobby's `max_moves`): an otherwise-ongoing position is then adjudicated
    ///   by material instead of being reported as ongoing.
    pub fn game_result_adjudicated(&self, capped: bool) -> u8 {
        match self.game_result() {
            0 if capped => self.adjudicate_material(),
            3 if self.termination_reason() == "move_limit" => self.adjudicate_material(),
            r => r,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::board::Board;

    /// `Board.history` stores PRIOR positions only, so threefold = 2 prior
    /// occurrences (+ current) and fivefold = 4 prior occurrences.
    #[test]
    fn repetition_thresholds_use_prior_only_history() {
        let mut b = Board::start_pos();
        let z = b.state.zobrist;
        assert_eq!(b.repetition_count(), 0);
        assert!(!b.is_game_over());

        b.history.push(z); // 1 prior + current = 2 total → not a draw
        assert!(!b.is_game_over());
        assert_eq!(b.termination_reason(), "ongoing");

        b.history.push(z); // 2 prior + current = 3 total → threefold
        assert!(b.is_game_over());
        assert_eq!(b.termination_reason(), "threefold_repetition");
        assert_eq!(b.game_result(), 3);

        b.history.push(z);
        b.history.push(z); // 4 prior + current = 5 total → fivefold
        assert_eq!(b.termination_reason(), "fivefold_repetition");
    }

    /// Same thresholds reached through real moves: two knight out-and-back
    /// shuffles bring the start position to its 3rd total occurrence.
    #[test]
    fn threefold_by_actual_knight_shuffle() {
        let mut b = Board::start_pos();
        let shuffle = ["d2c4", "d11c9", "c4d2", "c9d11"];
        for _ in 0..2 {
            for uci in shuffle {
                assert!(
                    !b.is_game_over(),
                    "game ended early at {uci}: {}",
                    b.termination_reason()
                );
                let m = b
                    .generate_moves()
                    .iter()
                    .copied()
                    .find(|m| m.stringify() == uci)
                    .unwrap_or_else(|| panic!("move {uci} not legal"));
                b.apply_move(m);
            }
        }
        // Start position now occurred 3 times (start + after each shuffle).
        assert_eq!(b.repetition_count(), 2);
        assert!(b.is_game_over());
        assert_eq!(b.termination_reason(), "threefold_repetition");
    }

    /// Material adjudication: the side up a piece wins a move-limit ending;
    /// balanced material stays a draw; genuine draws are never overturned.
    #[test]
    fn material_adjudication_at_move_limit() {
        // Kings + white rook (c1): White is +500cp.
        let up_rook = "12/12/12/12/12/6g5/12/12/12/12/6G5/2C9";
        let b = Board::from_hfen(&format!("{up_rook} w - - 0 1")).unwrap();
        assert_eq!(b.material_diff(), 500);
        assert_eq!(b.adjudicate_material(), 1);
        assert_eq!(b.game_result(), 0); // ongoing position…
        assert_eq!(b.game_result_adjudicated(false), 0); // …stays ongoing uncapped
        assert_eq!(b.game_result_adjudicated(true), 1); // …White wins at the cap

        // Same position at the internal no-progress limit (rule50 = 224):
        // draw by move_limit becomes a White win even without an external cap.
        let b = Board::from_hfen(&format!("{up_rook} w - - 224 120")).unwrap();
        assert_eq!(b.game_result(), 3);
        assert_eq!(b.termination_reason(), "move_limit");
        assert_eq!(b.game_result_adjudicated(false), 1);

        // Kings + black queen: Black is -900cp → Black wins at the cap.
        let b = Board::from_hfen("12/12/12/12/12/6g5/2f9/12/12/12/6G5/12 w - - 0 1").unwrap();
        assert_eq!(b.material_diff(), -900);
        assert_eq!(b.game_result_adjudicated(true), 2);

        // Balanced start position: capped game is still a draw.
        let b = Board::start_pos();
        assert_eq!(b.material_diff(), 0);
        assert_eq!(b.game_result_adjudicated(true), 3);

        // Genuine draw (insufficient material, KvK) is never adjudicated away.
        let b = Board::from_hfen("12/12/12/12/12/6g5/12/12/12/12/6G5/12 w - - 0 1").unwrap();
        assert_eq!(b.termination_reason(), "insufficient_material");
        assert_eq!(b.game_result_adjudicated(true), 3);
    }
}
