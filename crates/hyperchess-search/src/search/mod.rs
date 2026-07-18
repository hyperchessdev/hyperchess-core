//! Shared search building blocks used by every CPU searcher.
//!
//! Before this module the alpha-beta, iterative-deepening and guided searchers
//! each carried their own private copies of quiescence search, MVV-LVA move
//! ordering and the terminal (mate / draw) scoring rule.  Those copies have been
//! consolidated here so there is a single source of truth:
//!
//! * [`ordering`]   — move ordering (MVV-LVA, TT-move, full-eval ordering).
//! * [`quiescence`] — the capture-only quiescence search.
//! * [`terminal`]   — scoring a node that has no legal moves.
//!
//! The concrete searchers ([`super::alphabeta`], [`super::iterative`],
//! [`super::guided_alphabeta`]) are thin negamax drivers that wire these pieces
//! together.

pub mod ordering;
pub mod quiescence;
pub mod see;
pub mod terminal;

pub use ordering::{
    history_bonus, history_penalty, mvv_lva_score, order_by_eval, order_full, order_tactical,
    promote_tt_move,
};
pub use quiescence::quiesce;
pub use see::{see, see_ge_zero};
pub use terminal::{is_mate_score, terminal_value, value_from_tt, value_to_tt};
