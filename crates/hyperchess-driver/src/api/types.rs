// SPDX-License-Identifier: GPL-3.0-or-later
// HyperChess Core — hyperchess-driver
// File: crates/hyperchess-driver/src/api/types.rs
// Version: 1.1.0
// Copyright (c) 2026 HyperChess Developer Team

//! Request/response DTOs for the API driver — see
//! docs/hyperchess-core-extraction-plan.md §6 for the design.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Body of `POST /board/fen-validate`.
#[derive(Debug, Deserialize, ToSchema)]
pub struct FenValidateRequest {
    /// A position in HFEN(-I) notation.
    pub fen: String,
}

/// Result of `POST /board/fen-validate`. Note that a *rejected* FEN is still a
/// `200 OK` here — validation failure is the endpoint's normal output, not an
/// error, so callers read `valid` rather than the HTTP status.
#[derive(Debug, Serialize, ToSchema)]
pub struct FenValidateResponse {
    /// Whether the submitted string parsed as a legal position.
    pub valid: bool,
    /// Parser diagnostic, present only when `valid` is `false`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Body of `POST /move/legal`.
#[derive(Debug, Deserialize, ToSchema)]
pub struct LegalMovesRequest {
    /// A position in HFEN(-I) notation.
    pub fen: String,
}

/// Result of `POST /move/legal`.
#[derive(Debug, Serialize, ToSchema)]
pub struct LegalMovesResponse {
    /// Legal moves in plain UCI notation (e.g. `"e2e4"`).
    pub moves: Vec<String>,
}

/// Body of `POST /move/best`. Every field except `fen` is optional; omitted
/// fields fall back to the server-side defaults documented on each one.
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
    /// Hard wall-clock search budget in milliseconds, for callers that need a
    /// time-bounded move instead of (or in addition to) a depth ceiling —
    /// e.g. a persona with a documented "thinks for at most N seconds"
    /// guarantee. When present, search is bounded by
    /// [`hyperchess_search::timed::SearchLimits::movetime`] /
    /// [`hyperchess_search::mcts::mcts_bounded`]'s `movetime_ms` instead of
    /// the plain depth-only [`hyperchess_search::make_searcher`] path. `depth`
    /// still applies as a ceiling when both are set. Only the algorithms
    /// `random`, `alphabeta`/`ab`, `iterative`/`id`, `mcts`,
    /// `strategic`/`strategic_like`, and `aggressive`/`pro`/`commercial`/
    /// `stockfish_like` currently support this; other algorithm names ignore
    /// `movetime_ms` and fall back to the depth-only path.
    #[serde(default)]
    pub movetime_ms: Option<u64>,
}

/// Result of `POST /move/best`.
#[derive(Debug, Serialize, ToSchema)]
pub struct BestMoveResponse {
    /// Best move found, in plain UCI notation.
    pub best_move: String,
    /// Static evaluation of the resulting position, in centipawns from the
    /// side-to-move's perspective (of the position *before* the move).
    pub eval_cp: i32,
}

/// Result of `GET /health`.
#[derive(Debug, Serialize, ToSchema)]
pub struct HealthResponse {
    /// Always `"ok"` — the endpoint is a liveness probe, so reaching the
    /// handler at all is the signal; there is no degraded state to report.
    pub status: &'static str,
}

/// Uniform error body returned with any non-2xx status from this API.
#[derive(Debug, Serialize, ToSchema)]
pub struct ErrorResponse {
    /// Human-readable failure description; not a stable machine-readable code.
    pub error: String,
}
