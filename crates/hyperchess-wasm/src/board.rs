//! WASM bindings for HyperChess — exposed via wasm-bindgen.

use hyperchess_rules::tools::Searcher;
use hyperchess_rules::Board;
use hyperchess_search::{
    AlphaBetaSearcher, GuidedAlphaBeta, GuidedIterative, IterativeSearcher, MctsSearcher,
    ProSearcher, SearchLimits, SearchProfile, ShredderStyleSearcher, TimedSearcher,
};
use wasm_bindgen::prelude::*;

// `init_panic_hook`/`Helper::init()` moved to lib.rs's single combined
// `#[wasm_bindgen(start)]` — wasm-bindgen only allows one `start` function
// per crate, and hyperchess_3d's scene module needs its own init
// (console_log) too. See lib.rs.

// ── Calibration ───────────────────────────────────────────────────────────────

/// Which new piece to calibrate.  JS passes `"eagle"` or `"hawk"`.
#[wasm_bindgen]
pub fn calibrate_piece(piece: &str, games: u32, depth: u32, max_half_moves: u32) -> f64 {
    let (wc, bc) = match piece.to_lowercase().as_str() {
        "eagle" => ('E', 'e'),
        "hawk" => ('H', 'h'),
        _ => return 0.5,
    };

    const BASE_FEN: &str =
        "12/ehrnbqkbnrhe/pppppppppppp/12/12/12/12/12/12/PPPPPPPPPPPP/EHRNBQKBNRHE/12 w KQkq - 0 1";

    let games = games as usize;
    let cap = max_half_moves as usize;
    let mut wins = 0.0f64;
    let mut total = 0.0f64;

    for i in 0..games {
        let white_side = i % 2 == 0;
        let modified = if white_side {
            BASE_FEN.replacen('Q', &wc.to_string(), 1)
        } else {
            BASE_FEN.replacen('q', &bc.to_string(), 1)
        };
        let board = match Board::from_hfen(&modified) {
            Ok(b) => b,
            Err(_) => continue,
        };

        let outcome = wasm_play_game(&board, depth, cap);
        let score = match outcome {
            0 => 0.5, // draw
            1 => {
                if white_side {
                    1.0
                } else {
                    0.0
                }
            } // white won
            2 => {
                if !white_side {
                    1.0
                } else {
                    0.0
                }
            } // black won
            _ => 0.5,
        };
        wins += score;
        total += 1.0;
    }

    if total == 0.0 {
        0.5
    } else {
        wins / total
    }
}

/// Returns 0=draw, 1=white wins, 2=black wins (from the initial position's perspective).
fn wasm_play_game(board: &Board, depth: u32, max_half_moves: usize) -> u8 {
    let mut b = board.clone();
    let initial_turn = b.turn();
    let mut searcher = AlphaBetaSearcher::new();

    for _ in 0..max_half_moves {
        let moves = b.generate_moves();
        if moves.is_empty() {
            // Checkmate: side to move has no moves.
            let loser = b.turn();
            return if loser == initial_turn { 2 } else { 1 };
        }
        if b.repetition_count() >= 2 {
            return 0;
        }
        let mv = searcher.best_move(&b, depth);
        if mv.is_null() {
            return 0;
        }
        b.apply_move(mv);
    }
    0 // cap reached → draw
}

#[wasm_bindgen]
pub struct WasmBoard {
    inner: Board,
}

impl Default for WasmBoard {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen]
impl WasmBoard {
    /// Create a board at the starting position.
    #[wasm_bindgen(constructor)]
    pub fn new() -> WasmBoard {
        WasmBoard {
            inner: Board::start_pos(),
        }
    }

    /// Create a board from a HFEN string. Returns null on parse error.
    pub fn from_hfen(hfen: &str) -> Option<WasmBoard> {
        Board::from_hfen(hfen).ok().map(|inner| WasmBoard { inner })
    }

    /// Returns all legal moves as a space-separated UCI string (e.g. "a3a4 b3b4 ...").
    pub fn legal_moves(&self) -> String {
        self.inner
            .generate_moves()
            .iter()
            .map(|m| m.stringify())
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Apply a move given as a UCI string. Returns false if the move is not legal.
    pub fn apply_move(&mut self, uci: &str) -> bool {
        let moves = self.inner.generate_moves();
        for m in moves.iter() {
            if m.stringify() == uci {
                self.inner.apply_move(*m);
                return true;
            }
        }
        false
    }

    /// Run alpha-beta search at the given depth and return the best move as UCI.
    /// Returns an empty string if there are no legal moves.
    pub fn best_move(&mut self, depth: u32) -> String {
        let mut searcher = AlphaBetaSearcher::new();
        let m = searcher.best_move(&self.inner, depth);
        if m.is_null() {
            String::new()
        } else {
            m.stringify()
        }
    }

    /// Run MCTS with the given number of simulations and return the best move as UCI.
    /// Returns an empty string if there are no legal moves.
    pub fn best_move_mcts(&mut self, simulations: u32) -> String {
        let mut searcher = MctsSearcher::new(simulations);
        let m = searcher.best_move(&self.inner, 0);
        if m.is_null() {
            String::new()
        } else {
            m.stringify()
        }
    }

    /// Returns the current position as a HFEN string.
    pub fn hfen(&self) -> String {
        self.inner.get_hfen()
    }

    /// Returns true if the game has ended (checkmate, stalemate, or draw).
    pub fn is_game_over(&self) -> bool {
        self.inner.is_game_over()
    }

    /// Returns the game result: 0=ongoing, 1=white wins, 2=black wins, 3=draw.
    pub fn result(&self) -> u8 {
        self.inner.game_result()
    }

    /// [`Self::result`] with material adjudication for move-limit endings:
    /// a draw by the internal no-progress rule — and, when `capped` is true, an
    /// ongoing position stopped by an external per-game move cap — is decided
    /// by material (≥ 1 pawn wins) instead of collapsing to a draw. Genuine
    /// draws (stalemate, repetition, insufficient material) are unchanged.
    pub fn result_adjudicated(&self, capped: bool) -> u8 {
        self.inner.game_result_adjudicated(capped)
    }

    /// Material-only adjudication of the current position: 1 = White wins,
    /// 2 = Black wins, 3 = draw (within one pawn of equal material).
    pub fn adjudicate_material(&self) -> u8 {
        self.inner.adjudicate_material()
    }

    /// Returns why the game ended: "checkmate", "stalemate", "move_limit",
    /// "fivefold_repetition", "threefold_repetition", "insufficient_material", or
    /// "ongoing". Delegates to `Board::termination_reason`, the single source of
    /// truth also used by the server's termination endpoint, so the WASM/local-game
    /// path reports the same specific reason as server-backed games.
    pub fn termination(&self) -> String {
        self.inner.termination_reason().to_string()
    }

    /// Returns whose turn it is: "white" or "black".
    pub fn turn(&self) -> String {
        match self.inner.turn() {
            hyperchess_rules::Player::White => "white".to_string(),
            hyperchess_rules::Player::Black => "black".to_string(),
        }
    }

    /// Returns true if the side to move is currently in check.
    pub fn in_check(&self) -> bool {
        self.inner.in_check()
    }

    /// Returns the square index (0-143) of the king for the side to move, or 255 if not found.
    pub fn king_square(&self) -> u8 {
        self.inner.king_sq(self.inner.turn()).0
    }

    /// Returns the board as a 144-byte Uint8Array for WebGPU evaluation.
    /// Each byte is the Piece discriminant: 0=empty, 1-8=white, 9-16=black.
    /// Piece order: P=1, N=2, B=3, R=4, Q=5, K=6, Eagle=7, Hawk=8 (+ 8 for black).
    pub fn encode(&self) -> Vec<u8> {
        use hyperchess_rules::core::sq::SQ;
        (0u8..144)
            .map(|i| self.inner.piece_at(SQ(i)) as u8)
            .collect()
    }

    /// Run iterative deepening (with transposition table) at the given depth.
    /// Better than `best_move` for the same depth budget — TT avoids re-searching
    /// positions seen at shallower iterations.
    pub fn best_move_iterative(&mut self, depth: u32) -> String {
        let mut searcher = IterativeSearcher::new();
        let m = searcher.best_move(&self.inner, depth);
        if m.is_null() {
            String::new()
        } else {
            m.stringify()
        }
    }

    /// Compatibility guided alpha-beta label. Uses the canonical timed search.
    pub fn best_move_guided(&mut self, depth: u32) -> String {
        let mut searcher = GuidedAlphaBeta::new();
        let m = searcher.best_move(&self.inner, depth);
        if m.is_null() {
            String::new()
        } else {
            m.stringify()
        }
    }

    /// Compatibility guided iterative label. Uses the canonical timed search.
    pub fn best_move_guided_iterative(&mut self, depth: u32) -> String {
        let mut searcher = GuidedIterative::new();
        let m = searcher.best_move(&self.inner, depth);
        if m.is_null() {
            String::new()
        } else {
            m.stringify()
        }
    }

    /// Run the stronger Shredder-style tactical profile.
    pub fn best_move_shredder(&mut self, depth: u32) -> String {
        let mut searcher = ShredderStyleSearcher::new();
        let m = searcher.best_move(&self.inner, depth);
        if m.is_null() {
            String::new()
        } else {
            m.stringify()
        }
    }

    /// Run the "commercial-grade" Pro profile (PVS, aspiration windows and the
    /// speculative pruning family — reverse futility, frontier futility, delta
    /// pruning). The strongest fixed-depth option.
    pub fn best_move_pro(&mut self, depth: u32) -> String {
        let mut searcher = ProSearcher::new();
        let m = searcher.best_move(&self.inner, depth);
        if m.is_null() {
            String::new()
        } else {
            m.stringify()
        }
    }

    /// Anytime iterative-deepening search with a **wall-clock budget** (and an
    /// optional node cap as a deterministic backstop).
    ///
    /// This is the recommended interactive search: it deepens until `max_depth`,
    /// the time budget, or the node cap is hit, then returns the best move from the
    /// deepest *completed* depth. The search watches the clock itself
    /// (`js_sys::Date::now()`) — essential in the browser, where JS cannot interrupt
    /// a running synchronous WASM call, so an external timeout would never fire.
    ///
    /// * `movetime_ms == 0` → no time cap.
    /// * `node_limit == 0`  → no node cap.
    ///   (At least one of `movetime_ms` / `node_limit` / a small `max_depth` should
    ///   be set, or the search runs to `max_depth` which may be very deep.)
    pub fn best_move_timed(&mut self, max_depth: u32, movetime_ms: u32, node_limit: u32) -> String {
        use std::sync::atomic::AtomicBool;
        // WASM is single-threaded; nothing flips this mid-call. The time/node budgets
        // are the real bounds. The flag only satisfies the shared searcher API.
        let stop = AtomicBool::new(false);
        let mut searcher = TimedSearcher::new();
        let limits = SearchLimits {
            max_depth,
            movetime_ms: movetime_ms as u64,
            node_limit: node_limit as u64,
        };
        let m = searcher.search(&self.inner, &limits, &stop);
        if m.is_null() {
            String::new()
        } else {
            m.stringify()
        }
    }

    /// Anytime search with a wall-clock budget under a named profile:
    /// `"balanced"` (default), `"shredder"`, or `"pro"`. Same anytime semantics
    /// as [`Self::best_move_timed`] — this is what interactive Shredder/Pro
    /// callers should use so the computed move timeout is actually honoured
    /// (the fixed-depth `best_move_shredder` / `best_move_pro` wrappers ignore it).
    pub fn best_move_timed_profile(
        &mut self,
        max_depth: u32,
        movetime_ms: u32,
        node_limit: u32,
        profile: &str,
    ) -> String {
        use std::sync::atomic::AtomicBool;
        let profile = match profile.to_lowercase().as_str() {
            "shredder" | "shredder_style" | "shredder_like" => SearchProfile::ShredderStyle,
            "pro" | "commercial" | "stockfish_like" => SearchProfile::Pro,
            _ => SearchProfile::Balanced,
        };
        let stop = AtomicBool::new(false);
        let mut searcher = TimedSearcher::with_profile(profile);
        let limits = SearchLimits {
            max_depth,
            movetime_ms: movetime_ms as u64,
            node_limit: node_limit as u64,
        };
        let m = searcher.search(&self.inner, &limits, &stop);
        if m.is_null() {
            String::new()
        } else {
            m.stringify()
        }
    }

    /// Populate game history for draw-rule and search repetition checks.
    ///
    /// Contract (see `Board::repetition_count`): `history` stores **prior**
    /// positions only. Callers pass the full ordered HFEN list of the game
    /// (typically including the current position last); a trailing entry equal
    /// to the current position is dropped so it is never double-counted.
    /// Threefold then fires at `repetition_count() >= 2` (2 prior + current).
    pub fn set_hfen_history(&mut self, hfens: &str) {
        self.inner.history.clear();
        for hfen in hfens.lines() {
            let hfen = hfen.trim();
            if hfen.is_empty() {
                continue;
            }
            if let Ok(b) = Board::from_hfen(hfen) {
                self.inner.history.push(b.state.zobrist);
            }
        }
        // Drop the trailing "current position" entry, if the caller included it.
        if self.inner.history.last() == Some(&self.inner.state.zobrist) {
            self.inner.history.pop();
        }
    }

    /// Alias of [`Self::set_hfen_history`] — kept for older callers. Both draw
    /// rules and search now share the same prior-positions-only contract.
    pub fn set_search_history(&mut self, hfens: &str) {
        self.set_hfen_history(hfens);
    }
}
