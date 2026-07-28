//! Anytime iterative-deepening search with a real budget.
//!
//! This is the engine the interactive paths (WASM + server) should use. Unlike
//! [`IterativeSearcher`](crate::IterativeSearcher) it is **interruptible**:
//! it keeps the best move from the last *completed* depth and aborts the moment a
//! budget is exhausted, returning that move immediately.
//!
//! Three independent stop conditions are honoured so the same code runs everywhere:
//!
//! * **`movetime_ms`** — a hard wall-clock budget. Cross-platform: native uses
//!   `std::time::Instant`, WASM uses `js_sys::Date::now()` (browsers cannot
//!   interrupt a synchronous WASM call from JS, so the *search itself* must watch
//!   the clock — this is what makes the in-browser 30 s time limit actually work).
//! * **`node_limit`** — a hard cap on nodes searched. Deterministic and
//!   dependency-free; a useful secondary bound and the basis for reproducible tests.
//! * **`stop`** — an [`AtomicBool`] the caller can flip from another thread. The
//!   **server** spawns a watchdog thread that sets it after `movetime`, giving
//!   true wall-clock control without leaking the search thread.
//!
//! Move ordering combines the TT move, MVV-LVA captures, killers, a
//! countermove refutation table (indexed by the opponent's previous move) and
//! butterfly history with the HyperChess-specific "raptor bonus"
//! ([`crate::search::ordering::raptor_bonus`]) that tries Eagle/Hawk moves
//! into the enemy king's strike zone early — their jump checks cannot be
//! blocked, so they refute far more lines than history alone would predict.

use std::sync::atomic::{AtomicBool, Ordering};

/// Monotonic-ish millisecond clock that works on both native and `wasm32`.
/// `pub(crate)` so other bounded searches (e.g. MCTS) share the same clock.
pub(crate) mod clock {
    /// Current time in milliseconds. The absolute epoch is irrelevant — only
    /// differences are used (for elapsed-vs-budget comparisons).
    #[cfg(all(feature = "wasm", target_arch = "wasm32"))]
    #[inline]
    pub fn now_ms() -> f64 {
        // `performance.now()` isn't available in all worker contexts; `Date::now()`
        // always is and millisecond resolution is plenty for a move budget.
        js_sys::Date::now()
    }

    #[cfg(not(all(feature = "wasm", target_arch = "wasm32")))]
    #[inline]
    pub fn now_ms() -> f64 {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs_f64() * 1000.0)
            .unwrap_or(0.0)
    }
}

use crate::search::{
    history_bonus, history_penalty, order_by_eval, order_full, quiesce, terminal_value,
    value_from_tt, value_to_tt,
};
use hyperchess_rules::board::Board;
use hyperchess_rules::core::piece_move::HyperMove;
use hyperchess_rules::core::score::*;
use hyperchess_rules::tools::eval::evaluate;
use hyperchess_rules::tools::tt::{TTFlag, TranspositionTable};
use hyperchess_rules::tools::Searcher;

const HISTORY_SIZE: usize = 144 * 144;
const MAX_PLY: usize = 64;
/// How often (in nodes) we consult the `stop` flag. Power of two for a cheap mask.
const CHECK_INTERVAL: u64 = 2048;
/// Absolute ceiling on iterative-deepening depth (time/nodes bound it in practice).
const HARD_MAX_DEPTH: u32 = 64;
/// Late-move reductions kick in at this depth and after this many moves searched.
const LMR_MIN_DEPTH: i32 = 3;
const LMR_MIN_MOVE: u32 = 3;
/// Null-move pruning only applies at or above this depth.
const NULL_MOVE_MIN_DEPTH: i32 = 3;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SearchProfile {
    Balanced,
    Strategic,
    /// "Aggressive" profile modeled on the technique set of top engines
    /// (advanced engines class): the large TT plus the speculative pruning
    /// family — reverse futility, frontier futility and quiescence delta
    /// pruning. (PVS and aspiration windows are always on for every profile;
    /// they are pure speedups.)
    Aggressive,
}

impl SearchProfile {
    #[inline]
    fn tt_entries(self) -> usize {
        match self {
            SearchProfile::Balanced => 1 << 20,
            SearchProfile::Strategic | SearchProfile::Aggressive => 1 << 22,
        }
    }

    #[inline]
    fn uses_guided_root(self, depth: i32) -> bool {
        self == SearchProfile::Strategic && depth >= 3
    }

    #[inline]
    fn lmr_min_move(self) -> u32 {
        match self {
            SearchProfile::Balanced | SearchProfile::Aggressive => LMR_MIN_MOVE,
            SearchProfile::Strategic => LMR_MIN_MOVE + 1,
        }
    }

    /// Whether the speculative pruning family (reverse futility, frontier
    /// futility, quiescence delta pruning) is enabled. These trade a small
    /// tactical risk at frontier nodes for a much deeper effective search.
    #[inline]
    fn prunes(self) -> bool {
        self == SearchProfile::Aggressive
    }
}

/// True if `player` has at least one non-pawn, non-king piece. Used as a zugzwang
/// guard for null-move pruning: with only king + pawns, being forced to move can be
/// strictly bad, so the "pass and still win" assumption behind null-move breaks down.
#[inline]
fn has_non_pawn_material(board: &Board, player: hyperchess_rules::core::Player) -> bool {
    use hyperchess_rules::core::PieceType::*;
    for pt in [N, B, R, Q, E, H] {
        if board.piece_bb(player, pt).is_not_empty() {
            return true;
        }
    }
    false
}

/// Conservative LMR reduction: later moves at deeper nodes are reduced more, but
/// never below 1 ply and never so far that the reduced search would underflow.
#[inline]
fn lmr_reduction(depth: i32, move_count: u32) -> i32 {
    let mut r = 1;
    if move_count >= 6 {
        r += 1;
    }
    if depth >= 6 && move_count >= 10 {
        r += 1;
    }
    r.clamp(1, (depth - 2).max(1))
}

/// What bounds a search. `max_depth` is a ceiling; `movetime_ms`/`node_limit`/`stop`
/// truncate early. Any combination may be set; the first to trigger wins.
#[derive(Clone, Copy)]
pub struct SearchLimits {
    /// Iterative-deepening depth ceiling (clamped to [1, `HARD_MAX_DEPTH`]).
    pub max_depth: u32,
    /// Hard node cap. `0` = no node cap.
    pub node_limit: u64,
    /// Hard wall-clock budget in milliseconds. `0` = no time cap.
    pub movetime_ms: u64,
}

impl SearchLimits {
    pub fn depth(max_depth: u32) -> Self {
        Self {
            max_depth,
            node_limit: 0,
            movetime_ms: 0,
        }
    }
    pub fn nodes(max_depth: u32, node_limit: u64) -> Self {
        Self {
            max_depth,
            node_limit,
            movetime_ms: 0,
        }
    }
    /// Bound by wall-clock time (and a depth ceiling). This is the interactive default.
    pub fn movetime(max_depth: u32, movetime_ms: u64) -> Self {
        Self {
            max_depth,
            node_limit: 0,
            movetime_ms,
        }
    }
}

/// Reusable anytime searcher. Holds the TT/history/killers so a caller *may*
/// reuse it across moves in a game; constructing a fresh one per move is also fine.
pub struct TimedSearcher {
    tt: TranspositionTable,
    history: Box<[i32]>,
    killers: Box<[[HyperMove; 2]]>,
    /// Countermove table: `countermoves[prev.from*144 + prev.to]` holds the
    /// quiet move that most recently refuted (caused a beta cutoff against)
    /// the opponent move `prev`. A refutation tends to stay a refutation
    /// wherever the same opponent move appears in the tree.
    countermoves: Box<[HyperMove]>,
    profile: SearchProfile,
}

impl Default for TimedSearcher {
    fn default() -> Self {
        Self::new()
    }
}

impl TimedSearcher {
    pub fn new() -> Self {
        Self::with_profile(SearchProfile::Balanced)
    }

    pub fn strategic() -> Self {
        Self::with_profile(SearchProfile::Strategic)
    }

    /// The "commercial-grade" [`SearchProfile::Aggressive`] search.
    pub fn pro() -> Self {
        Self::with_profile(SearchProfile::Aggressive)
    }

    pub fn with_profile(profile: SearchProfile) -> Self {
        Self {
            tt: TranspositionTable::new(profile.tt_entries()),
            history: vec![0i32; HISTORY_SIZE].into_boxed_slice(),
            killers: vec![[HyperMove::null(); 2]; MAX_PLY].into_boxed_slice(),
            countermoves: vec![HyperMove::null(); HISTORY_SIZE].into_boxed_slice(),
            profile,
        }
    }

    /// Search `board` under `limits`, aborting if `stop` is set. Always returns a
    /// legal move when one exists (the best from the deepest completed depth, or
    /// the best-ordered root move if even depth 1 is cut short).
    pub fn search(&mut self, board: &Board, limits: &SearchLimits, stop: &AtomicBool) -> HyperMove {
        self.search_with_stats(board, limits, stop).best_move
    }

    pub fn search_with_stats(
        &mut self,
        board: &Board,
        limits: &SearchLimits,
        stop: &AtomicBool,
    ) -> SearchStats {
        let mut board = board.clone();
        let max_depth = limits.max_depth.clamp(1, HARD_MAX_DEPTH);

        let deadline = if limits.movetime_ms > 0 {
            Some(clock::now_ms() + limits.movetime_ms as f64)
        } else {
            None
        };

        let mut ctx = Ctx {
            tt: &mut self.tt,
            history: &mut self.history,
            killers: &mut self.killers,
            countermoves: &mut self.countermoves,
            nodes: 0,
            node_limit: limits.node_limit,
            deadline,
            stop,
            aborted: false,
            root_best: HyperMove::null(),
            profile: self.profile,
        };

        let mut best = HyperMove::null();
        let mut completed_depth = 0;
        let mut prev_score: Value = 0;
        for depth in 1..=max_depth {
            // Aspiration windows: from depth 4 search a narrow window around the
            // previous iteration's score and widen on failure. Most iterations
            // land inside the window, which raises cutoff rates everywhere; a
            // miss just re-searches with wider bounds, so results are unchanged.
            let (score, mv) = if depth >= 4 {
                let mut delta: Value = 50;
                let mut alpha = prev_score.saturating_sub(delta).max(-VALUE_INFINITE);
                let mut beta = prev_score.saturating_add(delta).min(VALUE_INFINITE);
                loop {
                    let (s, m) = search(
                        &mut board,
                        &mut ctx,
                        depth as i32,
                        alpha,
                        beta,
                        0,
                        HyperMove::null(),
                    );
                    if ctx.aborted {
                        break (s, m);
                    }
                    if s <= alpha {
                        alpha = s.saturating_sub(delta).max(-VALUE_INFINITE);
                    } else if s >= beta {
                        beta = s.saturating_add(delta).min(VALUE_INFINITE);
                    } else {
                        break (s, m);
                    }
                    delta = delta.saturating_mul(2);
                }
            } else {
                search(
                    &mut board,
                    &mut ctx,
                    depth as i32,
                    -VALUE_INFINITE,
                    VALUE_INFINITE,
                    0,
                    HyperMove::null(),
                )
            };
            if ctx.aborted {
                break;
            }
            prev_score = score;
            completed_depth = depth;
            if !mv.is_null() {
                best = mv;
            }
            // A proven mate won't change with more depth — stop early.
            if score.abs() >= VALUE_MATE_IN_MAX_PLY {
                break;
            }
        }

        // Fallbacks so we never return null when a legal move exists.
        if best.is_null() {
            best = ctx.root_best;
        }
        if best.is_null() {
            if let Some(&m) = board.generate_moves().iter().next() {
                best = m;
            }
        }
        SearchStats {
            best_move: best,
            completed_depth,
            nodes: ctx.nodes,
            aborted: ctx.aborted,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SearchStats {
    pub best_move: HyperMove,
    pub completed_depth: u32,
    pub nodes: u64,
    pub aborted: bool,
}

/// The canonical search is also a plain [`Searcher`]: a depth-bounded call with no
/// time/node cap. This lets `TimedSearcher` drop into anywhere a `Searcher` is
/// expected (so the whole engine shares one search implementation).
impl Searcher for TimedSearcher {
    fn best_move(&mut self, board: &Board, depth: u32) -> HyperMove {
        let stop = AtomicBool::new(false);
        self.search(board, &SearchLimits::depth(depth), &stop)
    }

    fn name(&self) -> &str {
        "Timed"
    }
}

struct Ctx<'a> {
    tt: &'a mut TranspositionTable,
    history: &'a mut [i32],
    killers: &'a mut [[HyperMove; 2]],
    countermoves: &'a mut [HyperMove],
    nodes: u64,
    node_limit: u64,
    /// Absolute wall-clock deadline (ms, same epoch as `clock::now_ms`), if any.
    deadline: Option<f64>,
    stop: &'a AtomicBool,
    aborted: bool,
    /// Best root move seen so far this iteration (used if the iteration aborts).
    root_best: HyperMove,
    profile: SearchProfile,
}

impl Ctx<'_> {
    #[inline]
    fn tick(&mut self) {
        self.nodes += 1;
        if self.node_limit != 0 && self.nodes >= self.node_limit {
            self.aborted = true;
        } else if self.nodes & (CHECK_INTERVAL - 1) == 0 {
            // Only consult the (relatively expensive) clock / atomic every
            // CHECK_INTERVAL nodes to keep the hot path cheap.
            if self.stop.load(Ordering::Relaxed) {
                self.aborted = true;
            } else if let Some(deadline) = self.deadline {
                if clock::now_ms() >= deadline {
                    self.aborted = true;
                }
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn search(
    board: &mut Board,
    ctx: &mut Ctx,
    depth: i32,
    mut alpha: Value,
    beta: Value,
    ply: usize,
    prev: HyperMove, // the move that led to this node (null at the root / after a null move)
) -> (Value, HyperMove) {
    if ctx.aborted {
        return (alpha, HyperMove::null());
    }
    ctx.tick();
    if ctx.aborted {
        return (alpha, HyperMove::null());
    }

    // `history` stores prior positions. Two prior occurrences plus the current
    // position is threefold; treating a single prior occurrence as a draw makes the
    // search overvalue move repetition.
    if board.repetition_count() >= 2 {
        return (VALUE_DRAW, HyperMove::null());
    }

    let in_check = board.in_check();

    // Rule-based draws (mirrors `Board::is_game_over`): the scaled 50-move rule
    // (224 half-moves on 12×12) and insufficient material. Checkmate takes
    // precedence over the move rule, so in check the draw only counts when an
    // evasion exists. The root is exempt so a move is always produced.
    if ply > 0 {
        if board.state.rule50 >= 224 && (!in_check || !board.generate_moves().is_empty()) {
            return (VALUE_DRAW, HyperMove::null());
        }
        if board.insufficient_material() {
            return (VALUE_DRAW, HyperMove::null());
        }
    }

    // Check extension: searching one ply deeper when in check resolves tactics that
    // would otherwise be cut off mid-combination. Bounded by the repetition draw and
    // the node/time budget, so perpetual checks can't loop forever.
    let depth = if in_check { depth + 1 } else { depth };

    if depth <= 0 {
        let delta_prune = ctx.profile.prunes();
        let score = quiesce(board, alpha, beta, ply, delta_prune, &mut || {
            ctx.tick();
            ctx.aborted
        });
        return (score, HyperMove::null());
    }

    let key = board.state.zobrist;
    // Mate scores are measured from the search root (not the absolute game ply),
    // keeping them inside VALUE_MATE_IN_MAX_PLY however long the game has run.
    let ply_val = ply as Value;
    let ply_clamped = ply.min(MAX_PLY - 1);

    let mut tt_move = HyperMove::null();
    if let Some(entry) = ctx.tt.probe(key) {
        tt_move = entry.best_move;
        // No TT cutoffs at the root: the stored move is returned unvalidated on a
        // cutoff, and the root move gets played — a (however unlikely) key collision
        // must never inject an illegal move. Deeper nodes only consume the score.
        if entry.depth >= depth && ply > 0 {
            let tt_score = value_from_tt(entry.score, ply_val);
            match entry.flag {
                TTFlag::Exact => return (tt_score, entry.best_move),
                TTFlag::LowerBound => {
                    if tt_score >= beta {
                        return (tt_score, entry.best_move);
                    }
                }
                TTFlag::UpperBound => {
                    if tt_score <= alpha {
                        return (tt_score, entry.best_move);
                    }
                }
            }
        }
    }

    // Reverse futility (static null move, Aggressive profile): at a shallow node whose
    // static eval is already comfortably above beta, an actual search will almost
    // surely fail high too. Never in check, at the root, or near mate bounds.
    let prunes = ctx.profile.prunes();
    if prunes && !in_check && ply > 0 && depth <= 3 && beta.abs() < VALUE_MATE_IN_MAX_PLY {
        let static_eval = evaluate(board);
        if static_eval - 120 * depth >= beta {
            return (beta, HyperMove::null());
        }
    }

    // Null-move pruning: if passing the turn still leaves us with a position good
    // enough to beat beta (searched at reduced depth), the real best move almost
    // certainly does too, so we can prune. Guarded against the cases where passing
    // is illegal-in-spirit: in check, at the root, near a mate bound, or in a likely
    // zugzwang (side to move has only king + pawns, where being forced to move hurts).
    if !in_check
        && ply > 0
        && depth >= NULL_MOVE_MIN_DEPTH
        && beta.abs() < VALUE_MATE_IN_MAX_PLY
        && has_non_pawn_material(board, board.turn())
    {
        let r = if depth >= 6 { 3 } else { 2 };
        board.apply_null_move();
        // `prev` is null for the child: there is no real move to learn a
        // countermove against, and a refutation of a *pass* is meaningless.
        let score = -search(
            board,
            ctx,
            depth - 1 - r,
            -beta,
            -beta + 1,
            ply + 1,
            HyperMove::null(),
        )
        .0;
        board.undo_null_move();
        if ctx.aborted {
            return (alpha, HyperMove::null());
        }
        if score >= beta {
            // Fail-hard: don't return an unproven mate score from a null search.
            return (beta, HyperMove::null());
        }
    }

    let moves = board.generate_moves();
    if moves.is_empty() {
        return (terminal_value(board, ply as Value), HyperMove::null());
    }

    // Countermove hint: the quiet move that last refuted the opponent's `prev`
    // anywhere in the tree. Indexed by (from, to) of `prev` — a butterfly
    // index, same shape as the history table.
    let counter = if prev.is_null() {
        HyperMove::null()
    } else {
        ctx.countermoves[prev.get_src().0 as usize * 144 + prev.get_dest().0 as usize]
    };

    let ordered = if ply == 0 && ctx.profile.uses_guided_root(depth) {
        let mut guided = order_by_eval(board, &moves);
        crate::search::promote_tt_move(&mut guided, tt_move);
        guided
    } else {
        order_full(
            board,
            &moves,
            tt_move,
            ctx.history,
            &ctx.killers[ply_clamped],
            counter,
        )
    };
    let mut best_move = ordered[0].0;
    let mut best_score = -VALUE_INFINITE;
    let mut flag = TTFlag::UpperBound;
    let mut quiet_tried: Vec<HyperMove> = Vec::new();
    let mut move_count: u32 = 0;

    // Frontier futility (Aggressive profile): at depth ≤ 2, a quiet move cannot
    // realistically lift a hopeless static eval past alpha — skip it. Never the
    // first move (a best move and bound must always exist) and never near mates.
    let futility_eval = if prunes && !in_check && depth <= 2 && alpha.abs() < VALUE_MATE_IN_MAX_PLY
    {
        Some(evaluate(board) + 150 * depth)
    } else {
        None
    };

    for (m, _) in &ordered {
        let m = *m;
        let is_quiet = !m.is_capture() && !m.is_promo();

        if let Some(fe) = futility_eval {
            if is_quiet && move_count > 0 && fe <= alpha {
                continue;
            }
        }

        board.apply_move(m);

        // Late-move reductions: deep into the move list, quiet moves that ordering
        // already ranked low are unlikely to beat alpha, so search them shallower
        // first. Only a move that *does* beat alpha is re-searched at full depth, so
        // the chosen move's score is always exact — LMR only saves time, never
        // changes correctness.
        let reduce = move_count >= ctx.profile.lmr_min_move()
            && depth >= LMR_MIN_DEPTH
            && is_quiet
            && !in_check
            && m != ctx.killers[ply_clamped][0]
            && m != ctx.killers[ply_clamped][1];

        let score = if reduce {
            let r = lmr_reduction(depth, move_count);
            let reduced = (depth - 1 - r).max(1);
            // Null-window probe at reduced depth.
            let probe = -search(board, ctx, reduced, -alpha - 1, -alpha, ply + 1, m).0;
            if probe > alpha && !ctx.aborted {
                // It might be good after all — re-search at full depth and window.
                -search(board, ctx, depth - 1, -beta, -alpha, ply + 1, m).0
            } else {
                probe
            }
        } else if move_count == 0 {
            // First move: full window — this is the PV candidate.
            -search(board, ctx, depth - 1, -beta, -alpha, ply + 1, m).0
        } else {
            // PVS: prove later moves worse than the PV with a cheap null-window
            // probe; only a fail-high *inside* the window earns a full-window
            // re-search (a probe ≥ beta is already a cutoff), so the chosen
            // move's score stays exact.
            let probe = -search(board, ctx, depth - 1, -alpha - 1, -alpha, ply + 1, m).0;
            if probe > alpha && probe < beta && !ctx.aborted {
                -search(board, ctx, depth - 1, -beta, -alpha, ply + 1, m).0
            } else {
                probe
            }
        };

        board.undo_move();
        move_count += 1;

        // If we were cut off mid-search, stop without trusting this (partial) child.
        if ctx.aborted {
            break;
        }

        if score > best_score {
            best_score = score;
            best_move = m;
        }

        if score > alpha {
            alpha = score;
            flag = TTFlag::Exact;
            if ply == 0 {
                ctx.root_best = m;
            }
            if alpha >= beta {
                flag = TTFlag::LowerBound;
                if !m.is_capture() {
                    history_bonus(ctx.history, m, depth);
                    for &tried in &quiet_tried {
                        history_penalty(ctx.history, tried, depth);
                    }
                    let k = &mut ctx.killers[ply_clamped];
                    if k[0] != m {
                        k[1] = k[0];
                        k[0] = m;
                    }
                    // Remember this quiet as the refutation of the opponent's
                    // previous move (countermove heuristic).
                    if !prev.is_null() {
                        ctx.countermoves
                            [prev.get_src().0 as usize * 144 + prev.get_dest().0 as usize] = m;
                    }
                }
                break;
            }
        }

        if !m.is_capture() {
            quiet_tried.push(m);
        }
    }

    // Don't pollute the TT with results from an aborted (incomplete) node.
    if !ctx.aborted {
        ctx.tt.store(
            key,
            best_move,
            value_to_tt(best_score, ply_val),
            depth,
            flag,
        );
    }
    (best_score, best_move)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicBool;

    #[test]
    fn returns_legal_move_from_start() {
        let board = Board::start_pos();
        let stop = AtomicBool::new(false);
        let mut s = TimedSearcher::new();
        let mv = s.search(&board, &SearchLimits::depth(3), &stop);
        assert!(!mv.is_null());
        // Move must be legal.
        assert!(board.generate_moves().iter().any(|&m| m == mv));
    }

    #[test]
    fn node_limit_truncates_but_still_returns_a_move() {
        let board = Board::start_pos();
        let stop = AtomicBool::new(false);
        let mut s = TimedSearcher::new();
        // Tiny budget: must abort deep search yet still hand back a legal move.
        let mv = s.search(&board, &SearchLimits::nodes(64, 100), &stop);
        assert!(!mv.is_null());
        assert!(board.generate_moves().iter().any(|&m| m == mv));
    }

    #[test]
    fn preset_stop_flag_returns_best_ordered_move() {
        let board = Board::start_pos();
        let stop = AtomicBool::new(true); // already stopped
        let mut s = TimedSearcher::new();
        let mv = s.search(&board, &SearchLimits::depth(20), &stop);
        assert!(!mv.is_null());
        assert!(board.generate_moves().iter().any(|&m| m == mv));
    }

    #[test]
    fn finds_mate_in_one() {
        // White: Qc11, Kc10. Black: Ka12. Qc11–b11 is mate (queen defended by
        // the king; a11/b12/b11 all covered). Exercises evasion-aware
        // quiescence, root-relative mate scores and the early mate stop.
        let board = Board::from_hfen("k11/2Q9/2K9/12/12/12/12/12/12/12/12/12 w - - 0 1")
            .expect("mate-in-1 HFEN should parse");
        let stop = AtomicBool::new(false);
        let mut s = TimedSearcher::new();
        let mv = s.search(&board, &SearchLimits::depth(4), &stop);
        assert_eq!(mv.stringify(), "c11b11");
    }

    #[test]
    fn node_limit_counts_quiescence_nodes() {
        let board = Board::start_pos();
        let stop = AtomicBool::new(false);
        let mut s = TimedSearcher::new();
        let stats = s.search_with_stats(&board, &SearchLimits::nodes(64, 500), &stop);
        // Ticks abort the instant the cap is reached — including inside
        // quiescence — so the count can never materially overshoot.
        assert!(stats.aborted);
        assert!(stats.nodes <= 500, "nodes = {}", stats.nodes);
    }

    #[test]
    fn pro_profile_finds_mate_in_one() {
        // The speculative pruning family must never prune away a forced mate:
        // reverse futility and futility are guarded off near mate bounds, and
        // delta pruning is bypassed by the evasion search.
        let board = Board::from_hfen("k11/2Q9/2K9/12/12/12/12/12/12/12/12/12 w - - 0 1")
            .expect("mate-in-1 HFEN should parse");
        let stop = AtomicBool::new(false);
        let mut s = TimedSearcher::pro();
        let mv = s.search(&board, &SearchLimits::depth(4), &stop);
        assert_eq!(mv.stringify(), "c11b11");
    }

    #[test]
    fn pro_profile_returns_legal_move_from_start() {
        let board = Board::start_pos();
        let stop = AtomicBool::new(false);
        let mut s = TimedSearcher::pro();
        let mv = s.search(&board, &SearchLimits::depth(5), &stop);
        assert!(!mv.is_null());
        assert!(board.generate_moves().iter().any(|&m| m == mv));
    }

    #[test]
    fn rule_draw_position_still_returns_a_root_move() {
        // 224 half-moves reached: interior nodes score it as a draw, but the
        // root is exempt so the engine still produces a legal move.
        let mut board = Board::start_pos();
        board.state.rule50 = 224;
        let stop = AtomicBool::new(false);
        let mut s = TimedSearcher::new();
        let mv = s.search(&board, &SearchLimits::depth(3), &stop);
        assert!(!mv.is_null());
        assert!(board.generate_moves().iter().any(|&m| m == mv));
    }
}
