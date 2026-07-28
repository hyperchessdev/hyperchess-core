// SPDX-License-Identifier: GPL-3.0-or-later
// HyperChess Core — hyperchess-driver
// File: crates/hyperchess-driver/src/api/mod.rs
// Version: 1.0.0
// Copyright (c) 2026 HyperChess Developer Team

//! Stateless REST/OpenAPI API driver — see
//! docs/hyperchess-core-extraction-plan.md §6 for the design.
//!
//! No DB, no auth server, no config volume, boots with zero required
//! environment variables. Search parameters (depth/simulations/algorithm) are
//! per-request; `ENGINE_DEFAULT_DEPTH`/`ENGINE_THREADS` env vars supply
//! defaults only.

mod handlers;
mod types;

use axum::routing::{get, post};
use axum::Router;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

#[derive(OpenApi)]
#[openapi(
    paths(
        handlers::health,
        handlers::fen_validate,
        handlers::legal_moves,
        handlers::best_move,
    ),
    components(schemas(
        types::HealthResponse,
        types::FenValidateRequest,
        types::FenValidateResponse,
        types::LegalMovesRequest,
        types::LegalMovesResponse,
        types::BestMoveRequest,
        types::BestMoveResponse,
        types::ErrorResponse,
    )),
    info(
        title = "HyperChess API",
        description = "Stateless REST API for HyperChess — legal move generation, FEN validation, and engine best-move queries. No account, no persistence: every request is self-contained.",
    )
)]
/// Marker type carrying the derived OpenAPI 3 document for this service.
struct ApiDoc;

/// Search depth used when a request omits `depth`, from `ENGINE_DEFAULT_DEPTH`.
fn default_depth() -> u32 {
    std::env::var("ENGINE_DEFAULT_DEPTH")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(4)
}

/// Applies `ENGINE_THREADS` to Rayon's global pool, if set.
///
/// Must run before the first parallel search: `build_global` can only succeed
/// once per process, and the failure is swallowed precisely because a pool
/// already installed by an earlier call is the acceptable outcome.
fn configure_threads() {
    if let Some(n) = std::env::var("ENGINE_THREADS")
        .ok()
        .and_then(|v| v.parse().ok())
    {
        rayon::ThreadPoolBuilder::new()
            .num_threads(n)
            .build_global()
            .ok();
    }
}

/// Builds the full route table, including the Swagger UI mount at `/docs`.
///
/// Split out from [`run`] so tests can exercise the routes without binding a
/// socket.
pub fn router() -> Router {
    // SwaggerUi::new(...).url(...) registers /openapi.json itself — a second
    // manual .route() for the same path panics at router-build time with
    // "Overlapping method route" (caught by an actual server smoke test, not
    // by cargo build/test — router construction only happens once the
    // server actually starts).
    Router::new()
        .route("/health", get(handlers::health))
        .route("/board/fen-validate", post(handlers::fen_validate))
        .route("/move/legal", post(handlers::legal_moves))
        .route("/move/best", post(handlers::best_move))
        .merge(SwaggerUi::new("/docs").url("/openapi.json", ApiDoc::openapi()))
}

/// Run the API server. Binds to `HOST` (default `0.0.0.0`) and `PORT`
/// (default `8080`).
pub async fn run() -> anyhow::Result<()> {
    hyperchess_rules::Helper::init();
    configure_threads();

    let host = std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(8080);

    let addr = format!("{host}:{port}");
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    println!("hyperchess api listening on http://{addr}  (docs: /docs, openapi: /openapi.json)");
    axum::serve(listener, router()).await?;
    Ok(())
}
