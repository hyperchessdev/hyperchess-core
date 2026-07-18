//! Monte Carlo Tree Search (UCT) for HyperChess.
//!
//! Uses static-evaluation rollouts instead of random playouts, which is far
//! stronger on a complex 12×12 board.  The `MctsSearcher` implements `Searcher`
//! so it can be dropped in wherever alpha-beta is used.
//!
//! An explicit `simulations` budget is authoritative. `simulations == 0` means
//! "auto": the `depth` argument passed through `Searcher::best_move` derives a
//! budget of `200 * 4^(depth-1)` so callers can use the same `--depth` knob for
//! both alpha-beta and MCTS.
//!
//! [`mcts_bounded`] additionally honours a wall-clock budget and a caller-owned
//! stop flag, mirroring [`TimedSearcher`](crate::TimedSearcher) — the
//! CLI/server move timeouts apply to MCTS through it.
//!
//! # GPU / batch usage
//! `mcts_with_eval` accepts an external leaf-evaluation closure so the CUDA
//! CLI can inject `gpu_batch_eval` for batched GPU leaf scoring. Leaves pending
//! evaluation carry a **virtual loss** so one batch explores distinct paths.

use rand::Rng;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::timed::clock;

use hyperchess_rules::board::Board;
use hyperchess_rules::core::piece_move::HyperMove;
use hyperchess_rules::tools::eval::evaluate;
use hyperchess_rules::tools::Searcher;

const C_UCT: f64 = 1.414; // √2

// ── Arena node ────────────────────────────────────────────────────────────────

pub struct MctsNode {
    pub mov: HyperMove,
    pub parent: Option<usize>,
    pub children: Vec<usize>,
    pub unexplored: Vec<HyperMove>,
    pub visits: u32,
    /// Cumulative score from **this node's side-to-move** perspective.
    pub value: f64,
    pub terminal: bool,
}

impl MctsNode {
    pub fn new(mov: HyperMove, parent: Option<usize>, board: &Board) -> Self {
        let moves = board.generate_moves();
        let terminal = moves.is_empty() || board.is_game_over();
        let mut unexplored: Vec<HyperMove> = moves.iter().copied().collect();
        // Shuffle so expansion order is random
        let mut rng = rand::thread_rng();
        unexplored.shuffle_rng(&mut rng);
        MctsNode {
            mov,
            parent,
            children: Vec::new(),
            unexplored,
            visits: 0,
            value: 0.0,
            terminal,
        }
    }

    /// UCB1 score seen from this node's **parent** (parent maximises this).
    /// `value/visits` is from THIS node's side — parent wants the opposite,
    /// so we negate for exploitation, add exploration bonus.
    #[inline]
    pub fn ucb1(&self, parent_visits: u32) -> f64 {
        if self.visits == 0 {
            return f64::INFINITY;
        }
        let exploit = -self.value / self.visits as f64;
        let explore = C_UCT * ((parent_visits as f64).ln() / self.visits as f64).sqrt();
        exploit + explore
    }
}

// Minimal shuffle helper (avoid importing SliceRandom in no-std contexts)
trait ShuffleExt {
    fn shuffle_rng<R: Rng>(&mut self, rng: &mut R);
}
impl ShuffleExt for Vec<HyperMove> {
    fn shuffle_rng<R: Rng>(&mut self, rng: &mut R) {
        let n = self.len();
        for i in (1..n).rev() {
            let j = rng.gen_range(0..=i);
            self.swap(i, j);
        }
    }
}

// ── Backpropagation ───────────────────────────────────────────────────────────

/// Walk from `leaf_idx` to root, alternating score sign at each level.
/// `leaf_score` is from the **leaf's side-to-move** perspective.
pub fn backprop(arena: &mut [MctsNode], leaf_idx: usize, leaf_score: f64) {
    let mut cur = leaf_idx;
    let mut s = leaf_score;
    loop {
        arena[cur].visits += 1;
        arena[cur].value += s;
        s = -s;
        match arena[cur].parent {
            Some(p) => cur = p,
            None => break, // reached root
        }
    }
}

// ── Terminal score helper ─────────────────────────────────────────────────────

/// Score at a terminal node from the **current side-to-move** perspective.
pub fn terminal_score(board: &Board) -> f64 {
    if board.generate_moves().is_empty() && board.in_check() {
        -1.0 // current side is mated
    } else {
        0.0 // stalemate / draw
    }
}

// ── SELECT + EXPAND ───────────────────────────────────────────────────────────

/// Walk the tree from root via UCB1 until an unexpanded or terminal node.
/// Returns (leaf_node_idx, board_at_leaf).
/// Mutates `arena` to add the expanded child.
pub fn select_expand(arena: &mut Vec<MctsNode>, root_board: &Board) -> (usize, Board) {
    let mut board = root_board.clone();
    let mut idx = 0usize;

    // SELECT: descend while fully expanded and non-terminal
    loop {
        if arena[idx].terminal || !arena[idx].unexplored.is_empty() {
            break;
        }
        let parent_v = arena[idx].visits;
        let best = arena[idx]
            .children
            .iter()
            .copied()
            .max_by(|&a, &b| {
                arena[a]
                    .ucb1(parent_v)
                    .partial_cmp(&arena[b].ucb1(parent_v))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .unwrap(); // children non-empty since unexplored is empty and not terminal
        board.apply_move(arena[best].mov);
        idx = best;
    }

    // EXPAND: if not terminal, add one unexplored child
    if !arena[idx].terminal && !arena[idx].unexplored.is_empty() {
        let mov = arena[idx].unexplored.pop().unwrap();
        board.apply_move(mov);
        let child = MctsNode::new(mov, Some(idx), &board);
        let child_idx = arena.len();
        arena.push(child);
        arena[idx].children.push(child_idx);
        idx = child_idx;
    }

    (idx, board)
}

// ── Virtual loss (batched selection) ─────────────────────────────────────────

/// Weight of a pending (in-flight) evaluation. A node's `value` is from its own
/// side-to-move perspective and its parent maximises `-value/visits`, so *adding*
/// to `value` makes the node less attractive to its parent — every node on a
/// pending path is de-prioritised until the real result replaces the loss.
const VIRTUAL_LOSS: f64 = 1.0;

fn add_virtual_loss(arena: &mut [MctsNode], leaf_idx: usize) {
    let mut cur = leaf_idx;
    loop {
        arena[cur].visits += 1;
        arena[cur].value += VIRTUAL_LOSS;
        match arena[cur].parent {
            Some(p) => cur = p,
            None => break,
        }
    }
}

fn remove_virtual_loss(arena: &mut [MctsNode], leaf_idx: usize) {
    let mut cur = leaf_idx;
    loop {
        arena[cur].visits -= 1;
        arena[cur].value -= VIRTUAL_LOSS;
        match arena[cur].parent {
            Some(p) => cur = p,
            None => break,
        }
    }
}

// ── Core MCTS (CPU, heuristic rollout) ───────────────────────────────────────

/// Run up to `simulations` MCTS iterations and return the best root move.
///
/// Two additional bounds mirror [`TimedSearcher`](crate::TimedSearcher):
/// `movetime_ms` is a hard wall-clock budget (`0` = no clock) and `stop` may be
/// flipped from another thread. Both are consulted every few simulations; on
/// abort the best move from the simulations completed so far is returned, and a
/// legal move is always returned when one exists.
pub fn mcts_bounded(
    board: &Board,
    simulations: u32,
    movetime_ms: u64,
    stop: &AtomicBool,
) -> HyperMove {
    /// How often (in simulations) the stop flag / deadline are consulted.
    const CHECK_INTERVAL: u32 = 64;

    let fallback = || {
        board
            .generate_moves()
            .iter()
            .next()
            .copied()
            .unwrap_or(HyperMove::null())
    };
    if simulations == 0 {
        return fallback();
    }

    let deadline = if movetime_ms > 0 {
        Some(clock::now_ms() + movetime_ms as f64)
    } else {
        None
    };

    let mut arena: Vec<MctsNode> = Vec::with_capacity((simulations as usize).min(1 << 20));
    arena.push(MctsNode::new(HyperMove::null(), None, board));

    for sim in 0..simulations {
        if sim % CHECK_INTERVAL == 0 {
            if stop.load(Ordering::Relaxed) {
                break;
            }
            if let Some(d) = deadline {
                if clock::now_ms() >= d {
                    break;
                }
            }
        }

        let (leaf_idx, leaf_board) = select_expand(&mut arena, board);

        let score = if arena[leaf_idx].terminal {
            terminal_score(&leaf_board)
        } else {
            // Heuristic rollout: static eval normalised to [-1, 1]
            (evaluate(&leaf_board) as f64).clamp(-3000.0, 3000.0) / 3000.0
        };

        backprop(&mut arena, leaf_idx, score);
    }

    let best = best_root_move(&arena);
    if best.is_null() {
        fallback()
    } else {
        best
    }
}

/// Run `simulations` MCTS iterations and return the best root move.
pub fn mcts(board: &Board, simulations: u32) -> HyperMove {
    mcts_bounded(board, simulations, 0, &AtomicBool::new(false))
}

/// Run batched MCTS using an external leaf evaluator (e.g. GPU batch eval).
///
/// `eval_fn` receives a slice of `(node_idx, board)` and must return a `Vec<f64>`
/// of scores in the same order, each in [-1, 1] from the board's side-to-move
/// perspective.
pub fn mcts_with_eval<F>(
    board: &Board,
    simulations: u32,
    batch_size: usize,
    eval_fn: F,
) -> HyperMove
where
    F: Fn(&[(usize, Board)]) -> Vec<f64>,
{
    mcts_with_eval_bounded(
        board,
        simulations,
        batch_size,
        0,
        &AtomicBool::new(false),
        eval_fn,
    )
}

/// [`mcts_with_eval`] with the same wall-clock/stop bounds as [`mcts_bounded`]:
/// `movetime_ms` is a hard budget (`0` = no clock) and `stop` may be flipped from
/// another thread. Both are consulted between batches — the natural granularity,
/// since a batch is one external-evaluator call — and on abort the best move from
/// the batches completed so far is returned.
pub fn mcts_with_eval_bounded<F>(
    board: &Board,
    simulations: u32,
    batch_size: usize,
    movetime_ms: u64,
    stop: &AtomicBool,
    eval_fn: F,
) -> HyperMove
where
    F: Fn(&[(usize, Board)]) -> Vec<f64>,
{
    let fallback = || {
        board
            .generate_moves()
            .iter()
            .next()
            .copied()
            .unwrap_or(HyperMove::null())
    };
    if simulations == 0 {
        return fallback();
    }

    let deadline = if movetime_ms > 0 {
        Some(clock::now_ms() + movetime_ms as f64)
    } else {
        None
    };

    let batch_size = batch_size.max(1);
    let mut arena: Vec<MctsNode> = Vec::with_capacity((simulations as usize).min(1 << 20));
    arena.push(MctsNode::new(HyperMove::null(), None, board));

    let mut pending: Vec<(usize, Board)> = Vec::with_capacity(batch_size);
    let mut sim = 0u32;

    while sim < simulations {
        if stop.load(Ordering::Relaxed) {
            break;
        }
        if let Some(d) = deadline {
            if clock::now_ms() >= d {
                break;
            }
        }

        // Fill batch. Each pending leaf carries a virtual loss so successive
        // selections within the same batch explore distinct paths instead of
        // all descending the identical UCB-best line.
        pending.clear();
        let current_batch = batch_size.min((simulations - sim) as usize);
        for _ in 0..current_batch {
            let (leaf_idx, leaf_board) = select_expand(&mut arena, board);
            add_virtual_loss(&mut arena, leaf_idx);
            pending.push((leaf_idx, leaf_board));
        }

        // Evaluate entire batch (GPU call if cuda, CPU otherwise)
        let scores = eval_fn(&pending);

        // Revert every virtual loss before applying real results, even if the
        // evaluator returned fewer scores than requested.
        for (leaf_idx, _) in pending.iter() {
            remove_virtual_loss(&mut arena, *leaf_idx);
        }

        // Backpropagate all. The evaluator contract is one score per pending
        // leaf; if it returns fewer (a partial GPU batch, say), the shorted
        // leaves fall back to CPU static eval instead of being silently dropped
        // — every selected leaf is backpropagated exactly once.
        for (i, (leaf_idx, leaf_board)) in pending.iter().enumerate() {
            let final_score = if arena[*leaf_idx].terminal {
                terminal_score(leaf_board)
            } else if let Some(&s) = scores.get(i) {
                s
            } else {
                (evaluate(leaf_board) as f64).clamp(-3000.0, 3000.0) / 3000.0
            };
            backprop(&mut arena, *leaf_idx, final_score);
        }

        sim += current_batch as u32;
    }

    let best = best_root_move(&arena);
    if best.is_null() {
        fallback()
    } else {
        best
    }
}

fn best_root_move(arena: &[MctsNode]) -> HyperMove {
    arena[0]
        .children
        .iter()
        .copied()
        .max_by_key(|&c| arena[c].visits)
        .map(|c| arena[c].mov)
        .unwrap_or(HyperMove::null())
}

// ── Searcher impl ─────────────────────────────────────────────────────────────

/// UCT Monte Carlo Tree Search.
pub struct MctsSearcher {
    /// Search budget (simulations per move).
    pub simulations: u32,
}

impl Default for MctsSearcher {
    fn default() -> Self {
        MctsSearcher { simulations: 800 }
    }
}

impl MctsSearcher {
    pub fn new(simulations: u32) -> Self {
        MctsSearcher { simulations }
    }
}

impl Searcher for MctsSearcher {
    fn best_move(&mut self, board: &Board, depth: u32) -> HyperMove {
        // An explicit simulation budget is authoritative — a caller asking for
        // 100 sims gets exactly 100. `simulations == 0` means "auto": derive the
        // budget from the shared --depth knob (200·4^(depth−1), e.g. depth 4 →
        // 12 800 sims) so one knob drives both alpha-beta and MCTS.
        let sims = if self.simulations > 0 {
            self.simulations
        } else if depth > 0 {
            200u32.saturating_mul(4u32.saturating_pow(depth.saturating_sub(1).min(5)))
        } else {
            Self::default().simulations
        };
        mcts(board, sims)
    }

    fn name(&self) -> &str {
        "MCTS"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_simulations_returns_a_legal_fallback_move() {
        let board = Board::start_pos();
        let mv = mcts(&board, 0);
        assert!(!mv.is_null());
        assert!(board.generate_moves().iter().any(|&m| m == mv));
    }

    #[test]
    fn zero_simulations_with_external_eval_returns_a_legal_fallback_move() {
        let board = Board::start_pos();
        let mv = mcts_with_eval(&board, 0, 32, |_| Vec::new());
        assert!(!mv.is_null());
        assert!(board.generate_moves().iter().any(|&m| m == mv));
    }

    #[test]
    fn preset_stop_flag_returns_a_legal_move_immediately() {
        let board = Board::start_pos();
        let stop = AtomicBool::new(true); // already stopped
        let mv = mcts_bounded(&board, 1_000_000, 0, &stop);
        assert!(!mv.is_null());
        assert!(board.generate_moves().iter().any(|&m| m == mv));
    }

    #[test]
    fn movetime_budget_truncates_but_returns_a_legal_move() {
        let board = Board::start_pos();
        let stop = AtomicBool::new(false);
        // 1 ms budget against a huge simulation count: the deadline check must
        // cut the loop short and still hand back a legal move.
        let mv = mcts_bounded(&board, 500_000, 1, &stop);
        assert!(!mv.is_null());
        assert!(board.generate_moves().iter().any(|&m| m == mv));
    }

    #[test]
    fn batched_eval_with_virtual_loss_returns_a_legal_move() {
        let board = Board::start_pos();
        let mv = mcts_with_eval(&board, 96, 16, |batch| vec![0.0; batch.len()]);
        assert!(!mv.is_null());
        assert!(board.generate_moves().iter().any(|&m| m == mv));
    }

    #[test]
    fn short_evaluator_result_still_backpropagates_every_leaf() {
        let board = Board::start_pos();
        // Evaluator violates its contract and returns half the requested scores;
        // the shorted leaves must be CPU-filled, not dropped, and the search must
        // still produce a legal move.
        let mv = mcts_with_eval(&board, 64, 16, |batch| vec![0.0; batch.len() / 2]);
        assert!(!mv.is_null());
        assert!(board.generate_moves().iter().any(|&m| m == mv));
    }

    #[test]
    fn bounded_batched_eval_preset_stop_returns_a_legal_move_immediately() {
        let board = Board::start_pos();
        let stop = AtomicBool::new(true); // already stopped
        let mv = mcts_with_eval_bounded(&board, 1_000_000, 64, 0, &stop, |batch| {
            vec![0.0; batch.len()]
        });
        assert!(!mv.is_null());
        assert!(board.generate_moves().iter().any(|&m| m == mv));
    }

    #[test]
    fn bounded_batched_eval_movetime_truncates_but_returns_a_legal_move() {
        let board = Board::start_pos();
        let stop = AtomicBool::new(false);
        let mv = mcts_with_eval_bounded(&board, 500_000, 64, 1, &stop, |batch| {
            vec![0.0; batch.len()]
        });
        assert!(!mv.is_null());
        assert!(board.generate_moves().iter().any(|&m| m == mv));
    }
}
