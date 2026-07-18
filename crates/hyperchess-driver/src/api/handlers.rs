//! Route handlers for the API driver.

use axum::extract::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

use hyperchess_rules::Board;

use super::types::{
    BestMoveRequest, BestMoveResponse, ErrorResponse, FenValidateRequest, FenValidateResponse,
    HealthResponse, LegalMovesRequest, LegalMovesResponse,
};

/// Wraps a parse/validation failure as a `400 Bad Request` JSON error body —
/// the only error condition any of these stateless handlers can hit.
pub(crate) struct ApiError(String);

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse { error: self.0 }),
        )
            .into_response()
    }
}

fn parse_board(fen: &str) -> Result<Board, ApiError> {
    Board::from_hfen(fen).map_err(ApiError)
}

#[utoipa::path(
    get,
    path = "/health",
    responses((status = 200, description = "Service is up", body = HealthResponse))
)]
pub async fn health() -> Json<HealthResponse> {
    Json(HealthResponse { status: "ok" })
}

#[utoipa::path(
    post,
    path = "/board/fen-validate",
    request_body = FenValidateRequest,
    responses((status = 200, description = "Validation result", body = FenValidateResponse))
)]
pub async fn fen_validate(Json(req): Json<FenValidateRequest>) -> Json<FenValidateResponse> {
    match Board::from_hfen(&req.fen) {
        Ok(_) => Json(FenValidateResponse {
            valid: true,
            error: None,
        }),
        Err(e) => Json(FenValidateResponse {
            valid: false,
            error: Some(e),
        }),
    }
}

#[utoipa::path(
    post,
    path = "/move/legal",
    request_body = LegalMovesRequest,
    responses(
        (status = 200, description = "Legal moves for the position", body = LegalMovesResponse),
        (status = 400, description = "Invalid FEN", body = ErrorResponse)
    )
)]
pub async fn legal_moves(
    Json(req): Json<LegalMovesRequest>,
) -> Result<Json<LegalMovesResponse>, ApiError> {
    let board = parse_board(&req.fen)?;
    let moves = board
        .generate_moves()
        .iter()
        .map(|m| m.stringify())
        .collect();
    Ok(Json(LegalMovesResponse { moves }))
}

#[utoipa::path(
    post,
    path = "/move/best",
    request_body = BestMoveRequest,
    responses(
        (status = 200, description = "Best move found", body = BestMoveResponse),
        (status = 400, description = "Invalid FEN", body = ErrorResponse)
    )
)]
pub async fn best_move(
    Json(req): Json<BestMoveRequest>,
) -> Result<Json<BestMoveResponse>, ApiError> {
    let board = parse_board(&req.fen)?;

    let algorithm = req.algorithm.as_deref().unwrap_or("alphabeta");
    let depth = req.depth.unwrap_or_else(super::default_depth);
    let simulations = req.simulations.unwrap_or(800);

    let mut searcher = hyperchess_search::make_searcher(algorithm, simulations);
    let mv = searcher.best_move(&board, depth);
    let eval_cp: i32 = hyperchess_rules::tools::eval::evaluate(&board);

    Ok(Json(BestMoveResponse {
        best_move: mv.stringify(),
        eval_cp,
    }))
}
