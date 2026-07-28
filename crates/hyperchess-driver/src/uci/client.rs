// SPDX-License-Identifier: GPL-3.0-or-later
// HyperChess Core — hyperchess-driver
// File: crates/hyperchess-driver/src/uci/client.rs
// Version: 1.0.0
// Copyright (c) 2026 HyperChess Developer Team

//! UCI (Universal Chess Interface) client.
//!
//! Manages a subprocess that speaks UCI (the native `hyperchess-uci` server)
//! and exposes typed async methods for analysis and legal-move generation.

use anyhow::{Context, Result};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};

/// A running UCI engine subprocess.
pub struct UciEngine {
    #[allow(dead_code)]
    child: Child,
    stdin: tokio::process::ChildStdin,
    stdout: BufReader<tokio::process::ChildStdout>,
}

impl UciEngine {
    /// Spawn a UCI engine and handshake (uci → uciok → isready → readyok).
    ///
    /// `extra_opts` is a list of (name, value) pairs sent as `setoption` commands
    /// before `isready`.
    pub async fn new(bin: &str, extra_opts: &[(&str, &str)]) -> Result<Self> {
        let mut child = Command::new(bin)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .spawn()
            .with_context(|| format!("failed to spawn UCI engine: {bin}"))?;

        let stdin = child.stdin.take().expect("stdin");
        let stdout = BufReader::new(child.stdout.take().expect("stdout"));
        let mut engine = Self {
            child,
            stdin,
            stdout,
        };

        engine.send("uci").await?;
        engine.wait_for("uciok").await?;

        for (name, value) in extra_opts {
            engine
                .send(&format!("setoption name {name} value {value}"))
                .await?;
        }

        engine.send("isready").await?;
        engine.wait_for("readyok").await?;

        Ok(engine)
    }

    /// Convenience: spawn the HyperChess native UCI server.
    ///
    /// `uci_bin` must be the path to the `hyperchess-uci` binary.
    pub async fn hyperchess_native(uci_bin: &str) -> Result<Self> {
        Self::new(uci_bin, &[]).await
    }

    /// Send a raw UCI command line.
    pub async fn send(&mut self, cmd: &str) -> Result<()> {
        self.stdin
            .write_all(format!("{cmd}\n").as_bytes())
            .await
            .context("write to UCI engine")?;
        self.stdin.flush().await.context("flush UCI engine")?;
        Ok(())
    }

    /// Read lines until one contains `token`; return that line.
    pub async fn wait_for(&mut self, token: &str) -> Result<String> {
        let mut line = String::new();
        loop {
            line.clear();
            let n = self
                .stdout
                .read_line(&mut line)
                .await
                .context("read from UCI engine")?;
            if n == 0 {
                anyhow::bail!("UCI engine closed stdout before sending '{token}'");
            }
            let trimmed = line.trim().to_string();
            if token.is_empty() || trimmed.contains(token) {
                return Ok(trimmed);
            }
        }
    }

    /// Read the next non-empty line from the engine.
    async fn read_line(&mut self) -> Result<String> {
        loop {
            let mut line = String::new();
            let n = self
                .stdout
                .read_line(&mut line)
                .await
                .context("read from UCI engine")?;
            if n == 0 {
                anyhow::bail!("UCI engine closed stdout unexpectedly");
            }
            let trimmed = line.trim().to_string();
            if !trimmed.is_empty() {
                return Ok(trimmed);
            }
        }
    }

    /// Analyse a position and return `(best_move_uci, score_cp, pv_moves)`.
    pub async fn analyse(
        &mut self,
        fen: &str,
        depth: u32,
        movetime_ms: u64,
    ) -> Result<(String, i32, Vec<String>)> {
        self.send(&format!("position fen {fen}")).await?;
        self.send(&format!("go depth {depth} movetime {movetime_ms}"))
            .await?;

        let mut score = 0i32;
        let mut pv: Vec<String> = vec![];

        let best = loop {
            let line = self.read_line().await?;
            if line.starts_with("info") {
                if let Some(s) = parse_score(&line) {
                    score = s;
                }
                if let Some(p) = parse_pv(&line) {
                    pv = p;
                }
            }
            if line.starts_with("bestmove") {
                break line.split_whitespace().nth(1).unwrap_or("0000").to_string();
            }
        };
        Ok((best, score, pv))
    }

    /// Return a soft-label probability distribution via MultiPV.
    ///
    /// Returns pairs `(uci_move, centipawn_score)` ordered by MultiPV rank.
    pub async fn multipv(&mut self, fen: &str, depth: u32, n: usize) -> Result<Vec<(String, i32)>> {
        self.send(&format!("setoption name MultiPV value {n}"))
            .await?;
        self.send(&format!("position fen {fen}")).await?;
        self.send(&format!("go depth {depth}")).await?;

        let mut moves: Vec<(String, i32)> = vec![];

        loop {
            let line = self.read_line().await?;
            if line.starts_with("info") && line.contains("multipv") {
                if let (Some(mv), Some(sc)) = (parse_pv_move(&line), parse_score(&line)) {
                    moves.push((mv, sc));
                }
            }
            if line.starts_with("bestmove") {
                break;
            }
        }

        self.send("setoption name MultiPV value 1").await?;
        Ok(moves)
    }

    /// Return legal moves in UCI notation via perft depth 1.
    pub async fn legal_moves(&mut self, fen: &str) -> Result<Vec<String>> {
        self.send(&format!("position fen {fen}")).await?;
        self.send("go perft 1").await?;

        let mut moves = vec![];
        loop {
            let line = self.read_line().await?;
            // Format: "a1b2: 1" for each legal move, then "Nodes searched: N"
            if line.starts_with("Nodes") || line.contains("Nodes searched") {
                break;
            }
            if let Some(mv) = line.split(':').next() {
                let mv = mv.trim().to_string();
                if !mv.is_empty() {
                    moves.push(mv);
                }
            }
        }
        Ok(moves)
    }

    /// Gracefully stop and quit the engine.
    pub async fn quit(&mut self) -> Result<()> {
        let _ = self.send("quit").await;
        Ok(())
    }
}

// ── UCI line parsers ──────────────────────────────────────────────────────────

fn parse_score(line: &str) -> Option<i32> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    let idx = parts.iter().position(|&s| s == "cp")?;
    parts.get(idx + 1)?.parse().ok()
}

fn parse_pv(line: &str) -> Option<Vec<String>> {
    let idx = line.find(" pv ")?;
    Some(
        line[idx + 4..]
            .split_whitespace()
            .map(|s| s.to_string())
            .collect(),
    )
}

fn parse_pv_move(line: &str) -> Option<String> {
    parse_pv(line)?.into_iter().next()
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_score() {
        let line = "info depth 5 seldepth 6 multipv 1 score cp 42 nodes 1234 pv a1b2 c3d4";
        assert_eq!(parse_score(line), Some(42));
    }

    #[test]
    fn test_parse_score_negative() {
        let line = "info depth 3 score cp -150 nodes 100 pv g2g4";
        assert_eq!(parse_score(line), Some(-150));
    }

    #[test]
    fn test_parse_pv() {
        let line = "info depth 5 score cp 10 nodes 500 pv e2e4 e7e5 g1f3";
        let pv = parse_pv(line).unwrap();
        assert_eq!(pv, vec!["e2e4", "e7e5", "g1f3"]);
    }

    #[test]
    fn test_parse_pv_move() {
        let line = "info multipv 1 score cp 30 pv g3g5 h3h5";
        assert_eq!(parse_pv_move(line), Some("g3g5".to_string()));
    }

    #[test]
    fn test_parse_score_no_cp() {
        // mate score — no "cp" token
        let line = "info depth 5 score mate 3 pv a1b2";
        assert_eq!(parse_score(line), None);
    }
}
