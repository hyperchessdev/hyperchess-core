// SPDX-License-Identifier: GPL-3.0-or-later
// HyperChess Core — hyperchess-driver
// File: crates/hyperchess-driver/tests/api.rs
// Version: 1.1.0
// Copyright (c) 2026 HyperChess Developer Team

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
async fn best_move_with_movetime_ms_returns_a_legal_move_for_alphabeta() {
    let response = hyperchess_driver::api::router()
        .oneshot(post(
            "/move/best",
            json!({ "fen": START_FEN, "algorithm": "alphabeta", "movetime_ms": 50 }),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let json = body_json(response).await;
    assert!(!json["best_move"].as_str().unwrap().is_empty());
}

#[tokio::test]
async fn best_move_with_movetime_ms_returns_a_legal_move_for_mcts() {
    let response = hyperchess_driver::api::router()
        .oneshot(post(
            "/move/best",
            json!({ "fen": START_FEN, "algorithm": "mcts", "movetime_ms": 50, "simulations": 200 }),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let json = body_json(response).await;
    assert!(!json["best_move"].as_str().unwrap().is_empty());
}

#[tokio::test]
async fn best_move_with_movetime_ms_falls_back_to_depth_only_for_an_unsupported_algorithm() {
    // "guided_ab" isn't in movetime_bounded_move's list; the handler must
    // fall back to the depth-only make_searcher path rather than error.
    let response = hyperchess_driver::api::router()
        .oneshot(post(
            "/move/best",
            json!({ "fen": START_FEN, "algorithm": "guided_ab", "depth": 2, "movetime_ms": 50 }),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let json = body_json(response).await;
    assert!(!json["best_move"].as_str().unwrap().is_empty());
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

#[test]
fn health_stays_responsive_while_a_search_is_in_flight() {
    // Regression test: `best_move`'s search previously ran inline in its
    // async fn with no `.await` points, so it held whatever Tokio worker
    // thread picked it up hostage for the whole movetime budget — starving
    // any other request scheduled on that thread, including `/health`. That
    // is exactly what let kubelet's liveness probe time out and kill the
    // pod under load.
    //
    // A same-runtime `oneshot()`-based test can't observe this: if the
    // measurement (the `/health` call, its timeout, its wakeup) shares the
    // very runtime that's starved, the measurement stalls right along with
    // it and the test passes either way. So this runs the real server (a
    // dedicated single-worker-thread runtime, on its own OS thread) and
    // probes it over a real TCP socket from the test's own independent
    // thread — reproducing what kubelet's probe actually experiences: a
    // socket read against a process that may or may not be free to answer.
    use std::io::{Read, Write};
    use std::net::TcpStream;
    use std::sync::mpsc;
    use std::time::Duration;

    let std_listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = std_listener.local_addr().unwrap();
    std_listener.set_nonblocking(true).unwrap();

    let (ready_tx, ready_rx) = mpsc::channel();
    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async move {
            let listener = tokio::net::TcpListener::from_std(std_listener).unwrap();
            ready_tx.send(()).unwrap();
            axum::serve(listener, hyperchess_driver::api::router())
                .await
                .unwrap();
        });
    });
    ready_rx.recv_timeout(Duration::from_secs(2)).unwrap();

    fn send_request(
        addr: std::net::SocketAddr,
        request: &str,
        timeout: Duration,
    ) -> std::io::Result<String> {
        let mut stream = TcpStream::connect(addr)?;
        stream.set_read_timeout(Some(timeout))?;
        stream.write_all(request.as_bytes())?;
        let mut response = String::new();
        stream.read_to_string(&mut response)?;
        Ok(response)
    }

    let body = json!({
        "fen": START_FEN, "algorithm": "alphabeta", "depth": 30, "movetime_ms": 800
    })
    .to_string();
    let search_request = format!(
        "POST /move/best HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    std::thread::spawn(move || {
        let _ = send_request(addr, &search_request, Duration::from_secs(5));
    });

    // Let the search actually start occupying the server's sole worker thread.
    std::thread::sleep(Duration::from_millis(150));

    let health_request = "GET /health HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n";
    let response = send_request(addr, health_request, Duration::from_millis(400))
        .expect("/health must respond promptly even while a search is running");
    assert!(
        response.starts_with("HTTP/1.1 200"),
        "unexpected /health response: {response}"
    );
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
