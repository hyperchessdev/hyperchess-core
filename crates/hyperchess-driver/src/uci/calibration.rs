//! Empirical piece-value calibration for Eagle and Hawk.
//!
//! Strategy (Phase 1, P1-T1/T2 from the implementation guideline):
//!
//! 1. Generate test positions where one side has a Queen replaced by the piece
//!    under test (equal-material positions so win-rate reflects piece strength).
//! 2. Play N engine self-play games at a fixed depth from each position.
//! 3. Measure win-rate for the piece-under-test side.
//! 4. Binary-search the value in [lo, hi] until win-rate is 0.50 ± `tolerance`.
//!
//! Since Fairy-Stockfish cannot run 12×12, calibration uses our native
//! `AlphaBetaSearcher`. Results are approximate but consistent with our eval.
//!
//! # Example
//! ```no_run
//! use hyperchess_driver::uci::calibration::{binary_search_value, PieceUnderTest};
//! let eagle_cp = binary_search_value(PieceUnderTest::Eagle, 200, 6, 0.05);
//! println!("Eagle ≈ {eagle_cp} cp");
//! ```

use hyperchess_rules::board::Board;
use hyperchess_rules::core::{PieceType, Player};
use hyperchess_rules::tools::eval::value_overrides;
use hyperchess_rules::tools::Searcher;
use hyperchess_search::AlphaBetaSearcher;

/// Which new piece is being calibrated.
#[derive(Clone, Copy, Debug)]
pub enum PieceUnderTest {
    Eagle,
    Hawk,
}

/// Outcome of a single game.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Outcome {
    Win,
    Draw,
    Loss,
}

/// Play a single game from `board` using AlphaBeta at `depth`.
///
/// Returns the outcome **from the perspective of the side that moved first**
/// (the `initial_turn` argument). Games are capped at `max_half_moves` to avoid
/// infinite loops; a draw is declared if no terminal position is reached.
fn play_game(board: &Board, depth: u32, max_half_moves: usize) -> Outcome {
    let mut b = board.clone();
    let initial_turn = b.turn();
    let mut searcher = AlphaBetaSearcher::new();

    for _ in 0..max_half_moves {
        let moves = b.generate_moves();
        if moves.is_empty() {
            // Terminal position. Check if the side to move is in check → checkmate.
            let side_to_move = b.turn();
            let loser = side_to_move;
            if loser == initial_turn {
                return Outcome::Loss;
            } else {
                return Outcome::Win;
            }
        }

        // 50-move / repetition draw.
        if b.repetition_count() >= 2 {
            return Outcome::Draw;
        }

        let mv = searcher.best_move(&b, depth);
        if mv.is_null() {
            return Outcome::Draw;
        }
        b.apply_move(mv);
    }

    Outcome::Draw
}

/// Play `games` games (alternating colours) from starting positions where
/// White has the piece-under-test instead of a Queen.
///
/// Returns win-rate = (wins + 0.5 * draws) / total for the side with the new piece.
pub fn calibrate_piece(piece: PieceUnderTest, games: usize, depth: u32) -> f64 {
    calibrate_piece_with_cap(piece, games, depth, 200)
}

fn calibrate_piece_with_cap(
    piece: PieceUnderTest,
    games: usize,
    depth: u32,
    max_half_moves: usize,
) -> f64 {
    let positions = generate_calibration_positions(piece, games);

    let mut wins = 0.0f64;
    let total = positions.len() as f64;

    for (board, piece_side) in &positions {
        let outcome = play_game(board, depth, max_half_moves);
        // play_game uses White-to-move start positions; Win = White won, Loss = Black won.
        let result = match outcome {
            Outcome::Win => {
                if *piece_side == Player::White {
                    1.0
                } else {
                    0.0
                }
            }
            Outcome::Loss => {
                if *piece_side == Player::Black {
                    1.0
                } else {
                    0.0
                }
            }
            Outcome::Draw => 0.5,
        };
        wins += result;
    }

    if total == 0.0 {
        0.5
    } else {
        wins / total
    }
}

/// Binary-search for the centipawn value of `piece` using self-play.
///
/// Searches in `[lo, hi]` until the range is narrower than 50 cp or
/// the win-rate is within `tolerance` of 0.50.
///
/// Returns the midpoint of the final bracket.
///
/// # Parameters
/// - `piece`     — Eagle or Hawk
/// - `games`     — games per candidate value (higher = more accurate, slower)
/// - `depth`     — search depth per move (3–5 is practical for calibration)
/// - `tolerance` — acceptable deviation from 0.50 (e.g. 0.05)
pub fn binary_search_value(piece: PieceUnderTest, games: usize, depth: u32, tolerance: f64) -> i32 {
    let pt = match piece {
        PieceUnderTest::Eagle => PieceType::E,
        PieceUnderTest::Hawk => PieceType::H,
    };

    let mut lo = 200i32; // weaker than a knight
    let mut hi = 1100i32; // stronger than a rook
    let mut converged: Option<i32> = None;

    while hi - lo > 50 {
        let mid = (lo + hi) / 2;
        // Apply the candidate value to the live evaluation so this iteration's
        // self-play games are actually played *with* `mid` — without this, every
        // iteration measures the same static value and the search is meaningless.
        value_overrides::set(pt, mid, mid);
        let wr = calibrate_piece(piece, games, depth);

        if wr > 0.50 + tolerance {
            hi = mid; // piece is too strong → lower its value
        } else if wr < 0.50 - tolerance {
            lo = mid; // piece is too weak → raise its value
        } else {
            converged = Some(mid);
            break;
        }
    }

    // Never leave a calibration override active after the run.
    value_overrides::clear();
    converged.unwrap_or((lo + hi) / 2)
}

/// Generate `n` calibration boards by substituting White's or Black's Queen
/// with the test piece in the HFEN starting position.
///
/// Alternates between White-has-piece and Black-has-piece to remove first-mover
/// bias. Positions come in pairs where the piece side swaps each game.
///
/// Starting HFEN ranks (top→bottom, rank 12→1):
///   `12/ehrnbqkbnrhe/pppppppppppp/12/12/12/12/12/12/PPPPPPPPPPPP/EHRNBQKBNRHE/12 w KQkq - 0 1`
/// White pieces (rank 2): EHRNBQKBNRHE — Q is the 6th character (index 5).
/// Black pieces (rank 11): ehrnbqkbnrhe — q is the 6th character (index 5).
fn generate_calibration_positions(piece: PieceUnderTest, n: usize) -> Vec<(Board, Player)> {
    let (white_char, black_char) = match piece {
        PieceUnderTest::Eagle => ('E', 'e'),
        PieceUnderTest::Hawk => ('H', 'h'),
    };

    // Base HFEN; split on space so we modify only the position part.
    const BASE_HFEN: &str =
        "12/ehrnbqkbnrhe/pppppppppppp/12/12/12/12/12/12/PPPPPPPPPPPP/EHRNBQKBNRHE/12 w KQkq - 0 1";

    let mut positions: Vec<(Board, Player)> = Vec::with_capacity(n);

    for i in 0..n {
        let piece_side = if i % 2 == 0 {
            Player::White
        } else {
            Player::Black
        };

        // Replace the queen belonging to `piece_side` with the test piece.
        let modified_hfen = if piece_side == Player::White {
            // White piece row contains capital Q — replace first occurrence.
            BASE_HFEN.replacen('Q', &white_char.to_string(), 1)
        } else {
            // Black piece row contains lowercase q — replace first occurrence.
            BASE_HFEN.replacen('q', &black_char.to_string(), 1)
        };

        match Board::from_hfen(&modified_hfen) {
            Ok(board) => positions.push((board, piece_side)),
            Err(e) => {
                // Should never happen with a well-formed HFEN; skip silently.
                eprintln!("calibration: HFEN parse error ({e}), skipping position");
            }
        }
    }

    positions
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The whole point of `binary_search_value`: a candidate override must
    /// change what the engine's evaluation reports. Uses an asymmetric position
    /// (White eagle vs. Black queen) so the eagle's value shifts the material
    /// balance rather than cancelling out.
    #[test]
    fn value_override_changes_evaluation() {
        hyperchess_rules::Helper::init();
        let (board, _) = generate_calibration_positions(PieceUnderTest::Eagle, 1)
            .into_iter()
            .next()
            .expect("calibration position");

        value_overrides::clear();
        let baseline = hyperchess_rules::tools::eval::evaluate(&board);
        value_overrides::set(PieceType::E, 1100, 1100);
        let boosted = hyperchess_rules::tools::eval::evaluate(&board);
        value_overrides::clear();
        let restored = hyperchess_rules::tools::eval::evaluate(&board);

        assert_ne!(
            baseline, boosted,
            "override must change eval (baseline {baseline}, boosted {boosted})"
        );
        assert_eq!(baseline, restored, "clear() must restore the static tables");
    }

    /// Fast smoke test: 1 game at depth 1, capped at 10 half-moves.
    #[test]
    fn calibrate_eagle_smoke() {
        hyperchess_rules::Helper::init();
        let wr = calibrate_piece_with_cap(PieceUnderTest::Eagle, 1, 1, 10);
        assert!((0.0..=1.0).contains(&wr), "win rate {wr} out of range");
    }

    #[test]
    fn calibrate_hawk_smoke() {
        hyperchess_rules::Helper::init();
        let wr = calibrate_piece_with_cap(PieceUnderTest::Hawk, 1, 1, 10);
        assert!((0.0..=1.0).contains(&wr), "win rate {wr} out of range");
    }

    /// Full calibration: realistic game count (slow, ignored by default).
    #[test]
    #[ignore = "slow — run with `-- --include-ignored` for calibration results"]
    fn calibrate_eagle_returns_sane_rate() {
        hyperchess_rules::Helper::init();
        let wr = calibrate_piece(PieceUnderTest::Eagle, 20, 4);
        assert!((0.0..=1.0).contains(&wr), "win rate {wr} out of range");
    }

    #[test]
    #[ignore = "slow — run with `-- --include-ignored` for calibration results"]
    fn calibrate_hawk_returns_sane_rate() {
        hyperchess_rules::Helper::init();
        let wr = calibrate_piece(PieceUnderTest::Hawk, 20, 4);
        assert!((0.0..=1.0).contains(&wr), "win rate {wr} out of range");
    }
}
