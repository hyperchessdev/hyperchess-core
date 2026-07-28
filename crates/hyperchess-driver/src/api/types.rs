//! Request/response DTOs for the API driver — see
//! docs/hyperchess-core-extraction-plan.md §6 for the design.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Deserialize, ToSchema)]
pub struct FenValidateRequest {
    /// A position in HFEN(-I) notation.
    pub fen: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct FenValidateResponse {
    pub valid: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct LegalMovesRequest {
    /// A position in HFEN(-I) notation.
    pub fen: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct LegalMovesResponse {
    /// Legal moves in plain UCI notation (e.g. `"e2e4"`).
    pub moves: Vec<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct BestMoveRequest {
    /// A position in HFEN(-I) notation.
    pub fen: String,
    /// Search algorithm. One of: random, alphabeta (ab), iterative (id),
    /// mcts, guided (guided_ab), guided_id, strategic, pro. Defaults to
    /// `alphabeta` if omitted.
    #[serde(default)]
    pub algorithm: Option<String>,
    /// Alpha-beta-family search depth. Defaults to the `ENGINE_DEFAULT_DEPTH`
    /// env var (itself defaulting to 4) if omitted.
    #[serde(default)]
    pub depth: Option<u32>,
    /// MCTS simulation count (only used by MCTS-family algorithms). Defaults
    /// to 800 if omitted.
    #[serde(default)]
    pub simulations: Option<u32>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct BestMoveResponse {
    /// Best move found, in plain UCI notation.
    pub best_move: String,
    /// Static evaluation of the resulting position, in centipawns from the
    /// side-to-move's perspective (of the position *before* the move).
    pub eval_cp: i32,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct HealthResponse {
    pub status: &'static str,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ErrorResponse {
    pub error: String,
}
