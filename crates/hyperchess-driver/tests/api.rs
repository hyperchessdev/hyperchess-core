//! Integration tests for the API driver — exercises the real axum `Router`
//! in-process (no live TCP listener needed) via `tower::ServiceExt::oneshot`,
//! the standard axum testing pattern. Complements (doesn't replace) the
//! actual `curl`-against-a-live-server smoke test run during development —
//! see docs/hyperchess-core-extraction-plan.md §12 Phase 5.

use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use serde_json::{json, Value};
use tower::ServiceExt;

const START_FEN: &str =
    "12/abcdefghijkl/mnopqrstuvwx/12/12/12/12/12/12/MNOPQRSTUVWX/ABCDEFGHIJKL/12 w - - 0 1";

async fn body_json(response: axum::response::Response) -> Value {
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

fn post(path: &str, body: Value) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(path)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap()
}

#[tokio::test]
async fn health_returns_ok() {
    let response = hyperchess_driver::api::router()
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let json = body_json(response).await;
    assert_eq!(json["status"], "ok");
}

#[tokio::test]
async fn fen_validate_accepts_a_real_position() {
    let response = hyperchess_driver::api::router()
        .oneshot(post("/board/fen-validate", json!({ "fen": START_FEN })))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let json = body_json(response).await;
    assert_eq!(json["valid"], true);
}

#[tokio::test]
async fn fen_validate_rejects_garbage() {
    let response = hyperchess_driver::api::router()
        .oneshot(post("/board/fen-validate", json!({ "fen": "not a fen" })))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK); // validation result, not a request error
    let json = body_json(response).await;
    assert_eq!(json["valid"], false);
    assert!(json["error"].is_string());
}

#[tokio::test]
async fn legal_moves_from_start_position_matches_perft_1() {
    let response = hyperchess_driver::api::router()
        .oneshot(post("/move/legal", json!({ "fen": START_FEN })))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let json = body_json(response).await;
    let moves = json["moves"].as_array().unwrap();
    // Cross-checked against hyperchess-search's own golden regression value
    // (tests/regression.rs::perft_start_golden asserts perft(&start, 1) == 62).
    assert_eq!(
        moves.len(),
        62,
        "legal move count from start must match perft(1)"
    );
}

#[tokio::test]
async fn legal_moves_rejects_invalid_fen_with_400() {
    let response = hyperchess_driver::api::router()
        .oneshot(post("/move/legal", json!({ "fen": "garbage" })))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let json = body_json(response).await;
    assert!(json["error"].is_string());
}

#[tokio::test]
async fn best_move_from_start_position_is_legal() {
    let response = hyperchess_driver::api::router()
        .oneshot(post(
            "/move/best",
            json!({ "fen": START_FEN, "algorithm": "alphabeta", "depth": 2 }),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let json = body_json(response).await;
    let mv = json["best_move"].as_str().unwrap();
    assert!(!mv.is_empty());
    // Symmetric start position: alpha-beta must score it dead even.
    assert_eq!(json["eval_cp"], 0);
}

#[tokio::test]
async fn best_move_defaults_to_alphabeta_when_algorithm_omitted() {
    let response = hyperchess_driver::api::router()
        .oneshot(post("/move/best", json!({ "fen": START_FEN, "depth": 2 })))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn openapi_json_is_served_and_matches_the_route_set() {
    let response = hyperchess_driver::api::router()
        .oneshot(
            Request::builder()
                .uri("/openapi.json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let json = body_json(response).await;
    let paths = json["paths"].as_object().unwrap();
    for expected in [
        "/health",
        "/board/fen-validate",
        "/move/legal",
        "/move/best",
    ] {
        assert!(
            paths.contains_key(expected),
            "missing path in openapi.json: {expected}"
        );
    }
}

#[tokio::test]
async fn docs_swagger_ui_is_mounted() {
    let response = hyperchess_driver::api::router()
        .oneshot(Request::builder().uri("/docs").body(Body::empty()).unwrap())
        .await
        .unwrap();
    // Swagger UI's index redirects to a trailing-slash path — a live curl
    // smoke test observed this as 303; any non-404/500 confirms the route
    // is actually mounted (this test's real purpose, after the Phase 5
    // "overlapping /openapi.json route" bug that only a live server run
    // caught — see mod.rs's router() comment).
    assert_ne!(response.status(), StatusCode::NOT_FOUND);
    assert_ne!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
}
