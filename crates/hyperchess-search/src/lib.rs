//! Search algorithms for HyperChess — alpha-beta, iterative deepening, MCTS,
//! and the anytime `TimedSearcher` used by the interactive (WASM + server)
//! paths. Depends on `hyperchess-rules` for board/move types; does not
//! depend on any driver (CLI/UCI/API) or WASM binding crate.

pub mod alphabeta;
pub mod guided_alphabeta;
pub mod iterative;
pub mod mcts;
pub mod pro;
pub mod search;
pub mod timed;

use hyperchess_rules::board::Board;
use hyperchess_rules::core::piece_move::HyperMove;
use hyperchess_rules::tools::Searcher;
use rand::Rng;

/// A bot that plays random legal moves.
pub struct RandomBot;

impl Searcher for RandomBot {
    fn best_move(&mut self, board: &Board, _depth: u32) -> HyperMove {
        let moves = board.generate_moves();
        if moves.is_empty() {
            return HyperMove::null();
        }
        let idx = rand::thread_rng().gen_range(0..moves.len());
        moves.as_slice()[idx]
    }

    fn name(&self) -> &str {
        "RandomBot"
    }
}

pub use alphabeta::AlphaBetaSearcher;
pub use guided_alphabeta::{GuidedAlphaBeta, GuidedIterative, StrategicSearcher};
pub use iterative::IterativeSearcher;
pub use mcts::MctsSearcher;
pub use pro::AggressiveSearcher;
pub use timed::{SearchLimits, SearchProfile, TimedSearcher};

/// Create a `Searcher` by name string.
/// Recognised: random, alphabeta (ab), iterative (id), mcts, strategic, pro.
pub fn make_searcher(name: &str, simulations: u32) -> Box<dyn Searcher> {
    match name.to_lowercase().as_str() {
        "random" => Box::new(RandomBot),
        "alphabeta" | "ab" => Box::new(AlphaBetaSearcher::new()),
        "iterative" | "id" => Box::new(IterativeSearcher::new()),
        "mcts" => Box::new(MctsSearcher::new(simulations)),
        "guided" | "guided_ab" => Box::new(GuidedAlphaBeta::new()),
        "guided_id" => Box::new(GuidedIterative::new()),
        "strategic" | "strategic_like" => Box::new(StrategicSearcher::new()),
        "aggressive" | "commercial" | "stockfish_like" => Box::new(AggressiveSearcher::new()),
        _ => {
            eprintln!("Unknown engine '{}', falling back to alphabeta", name);
            Box::new(AlphaBetaSearcher::new())
        }
    }
}

/// Re-exports matching the source repo's `hyperchess::bot_prelude` (formerly
/// re-exported from the rules crate root — see
/// docs/hyperchess-core-extraction-plan.md §12 Phase 3). Now lives on the
/// crate that actually owns these types instead of being re-exported by
/// hyperchess-rules, which must not depend on search.
pub mod prelude {
    pub use crate::make_searcher;
    pub use crate::AlphaBetaSearcher;
    pub use crate::IterativeSearcher;
    pub use crate::MctsSearcher;
    pub use crate::RandomBot;
    pub use crate::StrategicSearcher;
    pub use hyperchess_rules::tools::Searcher;
}
