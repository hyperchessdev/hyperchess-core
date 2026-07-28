// SPDX-License-Identifier: GPL-3.0-or-later
// HyperChess Core — hyperchess-driver
// File: crates/hyperchess-driver/src/cli/export.rs
// Version: 1.0.0
// Copyright (c) 2026 HyperChess Developer Team

//! Save game results: HFEN, detailed TXT stats, and JSON game record.

use hyperchess_rules::board::Board;
use hyperchess_rules::core::piece_identity::{
    identity_material_value, piece_from_identity, position_uses_identity,
};
use hyperchess_rules::core::Piece;
use std::fs;

// ── Move record (enriched) ────────────────────────────────────────────────────

/// All data captured for a single half-move (ply).
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct MoveRecord {
    pub ply: u32,
    pub side: &'static str, // "White" | "Black"
    pub move_str: String,
    /// HPGN-I string for this move (`<identity>:<uci>`, or plain UCI if no
    /// identity was tracked at the source square). See docs/HPGN-I-FORMAT.md.
    pub hpgn_i: String,
    pub hfen_after: String,
    pub eval_score: i32,         // centipawns, from White's perspective
    pub think_ms: u128,          // milliseconds spent on this move
    pub root_moves_count: usize, // how many root moves were considered
    pub backend: String,         // e.g. "GPU-MCTS(800)" | "AlphaBeta(d4)"
    /// Uniform random float in [0, 1) drawn from the seeded RNG at the exact
    /// moment the engine commits to its chosen move. Independent per ply.
    /// Use this for statistical analysis (distribution tests, entropy checks).
    pub decision_rand: f64,
}

// ── Game metadata ─────────────────────────────────────────────────────────────

/// Summary statistics for a completed game.
#[derive(Debug, Clone)]
pub struct GameStats {
    pub white_engine: String,
    pub black_engine: String,
    pub depth: u32,
    pub white_depth: u32,
    pub black_depth: u32,
    pub white_simulations: u32,
    pub black_simulations: u32,
    /// Skill level (1–20) for White, if set via --white-skill. None = not specified.
    pub white_skill: Option<u32>,
    /// Skill level (1–20) for Black, if set via --black-skill. None = not specified.
    pub black_skill: Option<u32>,
    pub result: String,
    pub total_moves: u32,
    pub gpu_name: Option<String>,
    pub game_seed: u64,
    pub moves: Vec<MoveRecord>,
    pub total_think_ms: u128,
    pub white_think_ms: u128,
    pub black_think_ms: u128,
    pub white_moves_on_gpu: usize,
    pub black_moves_on_gpu: usize,
}

impl GameStats {
    // Inherited from the source repo verbatim (10 params) — grouping into a
    // config struct would be a real API change, out of scope for this
    // copy-and-relocate phase. Same treatment as hyperchess-rules'
    // board/movegen/king.rs::can_castle in Phase 1.
    /// Start recording a game. Per-side depth/simulations are kept separate
    /// because White and Black may run different engines entirely; `depth` is
    /// the max of the two purely so single-number report lines have something
    /// to print.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        white_engine: &str,
        black_engine: &str,
        white_depth: u32,
        black_depth: u32,
        white_simulations: u32,
        black_simulations: u32,
        white_skill: Option<u32>,
        black_skill: Option<u32>,
        gpu_name: Option<String>,
        game_seed: u64,
    ) -> Self {
        GameStats {
            white_engine: white_engine.to_string(),
            black_engine: black_engine.to_string(),
            depth: white_depth.max(black_depth),
            white_depth,
            black_depth,
            white_simulations,
            black_simulations,
            white_skill,
            black_skill,
            result: String::new(),
            total_moves: 0,
            gpu_name,
            game_seed,
            moves: Vec::new(),
            total_think_ms: 0,
            white_think_ms: 0,
            black_think_ms: 0,
            white_moves_on_gpu: 0,
            black_moves_on_gpu: 0,
        }
    }

    /// Append one ply and fold its cost into the running totals.
    ///
    /// Side attribution is by the record's `side` string and GPU attribution by
    /// a `"GPU-"` backend prefix, so the aggregate counters stay correct even
    /// when a game mixes backends mid-run (CPU fallback after a GPU failure).
    pub fn push_move(&mut self, rec: MoveRecord) {
        self.total_think_ms += rec.think_ms;
        if rec.side == "White" {
            self.white_think_ms += rec.think_ms;
            if rec.backend.starts_with("GPU-") {
                self.white_moves_on_gpu += 1;
            }
        } else {
            self.black_think_ms += rec.think_ms;
            if rec.backend.starts_with("GPU-") {
                self.black_moves_on_gpu += 1;
            }
        }
        self.moves.push(rec);
    }
}

// ── Save helpers ──────────────────────────────────────────────────────────────

/// Build the canonical `hyperchess_rules::notation::GameRecord` for a finished
/// game — the shared class used to export both HFEN-I and HPGN-I.
fn to_game_record(stats: &GameStats) -> hyperchess_rules::notation::GameRecord {
    let mut rec =
        hyperchess_rules::notation::GameRecord::new(&stats.white_engine, &stats.black_engine);
    rec.event = "HyperChess CUDA Engine Selfplay".to_string();
    rec.site = "HyperChess".to_string();
    rec.date = chrono::Local::now().format("%Y.%m.%d").to_string();
    rec.result = match stats.result.as_str() {
        r if r.contains("White wins") => "1-0".to_string(),
        r if r.contains("Black wins") => "0-1".to_string(),
        _ => "1/2-1/2".to_string(),
    };
    for m in &stats.moves {
        rec.push_move(m.hpgn_i.clone());
    }
    rec
}

/// Save every HFEN-I position of the game (start position followed by each
/// move's resulting position), one per line.
pub fn save_hfeni(stats: &GameStats, path: &str) {
    let mut out = String::with_capacity(stats.moves.len() * 100 + 100);
    out.push_str(&Board::start_pos().get_hfen());
    out.push('\n');
    for rec in &stats.moves {
        out.push_str(&rec.hfen_after);
        out.push('\n');
    }
    match fs::write(path, &out) {
        Ok(_) => println!("HFEN-I → {}", path),
        Err(e) => eprintln!("Error saving HFEN-I: {}", e),
    }
}

/// Save the game history as canonical HPGN-I movetext (tags + identity-aware
/// moves). This replaces standard-chess PGN export: HyperChess's 12×12 board,
/// Eagle/Hawk pieces, and identity-tracked promotions have no faithful
/// encoding in classic PGN.
pub fn save_hpgni(stats: &GameStats, path: &str) {
    let rec = to_game_record(stats);
    match rec.save_hpgni(path) {
        Ok(_) => println!("HPGN-I → {}", path),
        Err(e) => eprintln!("Error saving HPGN-I: {}", e),
    }
}

/// Per-side play-quality scores for a finished game (the "v3.1 spec" block in
/// the exported stats). All components are in `[0, 1]`; only `v_target` is
/// signed.
///
/// These are derived purely from the engine's own eval trace, so they measure
/// self-consistency of play, not strength against an external reference.
#[derive(Debug, Clone)]
pub struct MasteryMetrics {
    /// `1 - mean(win-probability lost per own move)`. Falls as blunders mount.
    pub soundness: f64,
    /// Soundness plus a bonus per material sacrifice that did *not* cost win
    /// probability — rewards deliberate sacrifices over accidental losses.
    pub brilliance: f64,
    /// How quickly a won position was converted, once win probability first
    /// crossed 0.85. Halved if the advantage was ever given back below 0.5.
    pub efficiency: f64,
    /// How well the side held together while losing (win probability < 0.15).
    /// Left at 1.0 when the side was never in a lost position.
    pub resilience: f64,
    /// Aggregate of the four components, weighted by game outcome and
    /// penalised toward the weakest active component.
    pub mastery: f64,
    /// Game result mapped to a coarse band (loss/draw/win) and refined within
    /// that band by `mastery`; stays in `[0, 1]`.
    pub refined_outcome: f64,
    /// `refined_outcome` rescaled to `[-1, +1]` for use as a value-network
    /// training target.
    pub v_target: f64,
}

/// Sum one side's material from a HFEN board field.
///
/// Parsed by hand rather than through `Board` because this runs once per ply
/// over an already-recorded game. Handles both encodings the format allows:
/// identity letters (where the letter itself carries the value) and plain
/// piece chars, plus the inline `id:Type` pair a promoted pawn leaves behind —
/// there the promoted type is what counts, not the pawn.
fn get_material_value(hfen: &str, side: &str) -> i32 {
    fn type_value(piece: Piece) -> i32 {
        match piece {
            Piece::WhitePawn | Piece::BlackPawn => 100,
            Piece::WhiteKnight | Piece::BlackKnight => 320,
            Piece::WhiteBishop | Piece::BlackBishop => 330,
            Piece::WhiteRook | Piece::BlackRook => 500,
            Piece::WhiteQueen | Piece::BlackQueen => 900,
            Piece::WhiteEagle | Piece::BlackEagle => 700,
            Piece::WhiteHawk | Piece::BlackHawk => 550,
            Piece::WhiteKing | Piece::BlackKing | Piece::None => 0,
        }
    }

    let board_part = hfen.split_whitespace().next().unwrap_or(hfen);
    let identity_mode = position_uses_identity(board_part);
    let mut val = 0;
    let mut chars = board_part.chars().peekable();
    while let Some(c) = chars.next() {
        if c.is_ascii_digit() || c == '/' {
            continue;
        }
        // Inline `id:Type` promotion pair: one square whose current type is
        // the override — material counts the promoted type, not the pawn.
        let (piece, value) = if chars.peek() == Some(&':') {
            chars.next();
            let Some(t) = chars.next() else { continue };
            let Some(p) = Piece::from_char(t) else {
                continue;
            };
            (p, type_value(p))
        } else {
            let piece = if identity_mode {
                piece_from_identity(c).or_else(|| Piece::from_char(c))
            } else {
                Piece::from_char(c).or_else(|| piece_from_identity(c))
            };
            let Some(p) = piece else { continue };
            let v = if identity_mode {
                identity_material_value(c).unwrap_or(0)
            } else {
                type_value(p)
            };
            (p, v)
        };
        let is_side_piece = matches!(
            (side, piece.player()),
            ("White", Some(hyperchess_rules::core::Player::White))
                | ("Black", Some(hyperchess_rules::core::Player::Black))
        );
        if is_side_piece {
            val += value;
        }
    }
    val
}

/// Blend the four mastery components: half weighted mean, half worst active
/// component.
///
/// The min term is what stops a single strong dimension from carrying a
/// lopsided game — a player who is sound but never converts should not score
/// like one who did both. Weights depend on `result_code` (1 = won, 2 = drew,
/// anything else = lost) so a loss is judged mostly on resilience and a win
/// mostly on soundness; zero-weight components are excluded from the min.
fn weighted_penalized_min(sound: f64, brill: f64, eff: f64, resil: f64, result_code: i32) -> f64 {
    let w = match result_code {
        1 => [0.5, 0.2, 0.3, 0.0], // Winner
        2 => [0.4, 0.2, 0.2, 0.2], // Draw
        _ => [0.3, 0.0, 0.0, 0.7], // Loser
    };
    let x = [
        sound.clamp(0.0, 1.0),
        brill.clamp(0.0, 1.0),
        eff.clamp(0.0, 1.0),
        resil.clamp(0.0, 1.0),
    ];
    let mut w_sum = 0.0;
    let mut active_vals = Vec::new();
    for i in 0..4 {
        w_sum += w[i] * x[i];
        if w[i] > 0.0 {
            active_vals.push(x[i]);
        }
    }
    let p_min = if active_vals.is_empty() {
        0.0
    } else {
        active_vals
            .into_iter()
            .min_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap()
    };
    0.5 * w_sum + 0.5 * p_min
}

/// Compute `(white, black)` mastery metrics for a finished game.
///
/// Centipawn evals are mapped to win probability through a logistic with slope
/// 0.004/cp, and each side is scored on how much of its own win probability it
/// gave away per move. The final ply compares against the actual result rather
/// than a next-ply eval, so a decisive finish is credited as such.
///
/// An empty game yields perfect scores with a neutral 0.5 outcome, keeping
/// callers free of a special case.
pub fn compute_mastery(stats: &GameStats) -> (MasteryMetrics, MasteryMetrics) {
    let n = stats.moves.len();
    if n == 0 {
        let dummy = MasteryMetrics {
            soundness: 1.0,
            brilliance: 1.0,
            efficiency: 1.0,
            resilience: 1.0,
            mastery: 1.0,
            refined_outcome: 0.5,
            v_target: 0.0,
        };
        return (dummy.clone(), dummy);
    }

    let starting_hfen = Board::start_pos().get_hfen();

    let mut wp_white = Vec::with_capacity(n);
    let mut wp_black = Vec::with_capacity(n);

    for rec in &stats.moves {
        let val_white = if rec.side == "White" {
            rec.eval_score as f64
        } else {
            -rec.eval_score as f64
        };
        let wp = 1.0 / (1.0 + f64::exp(-0.004 * val_white));
        wp_white.push(wp);
        wp_black.push(1.0 - wp);
    }

    let p_term_white = match stats.result.as_str() {
        r if r.contains("White wins") => 1.0,
        r if r.contains("Black wins") => 0.0,
        _ => 0.5,
    };
    let p_term_black = 1.0 - p_term_white;

    let mut w_losses = Vec::new();
    let mut b_losses = Vec::new();
    let mut w_brilliant_count = 0;
    let mut b_brilliant_count = 0;
    let mut w_sac_attempted = false;
    let mut b_sac_attempted = false;

    for i in 0..n {
        let rec = &stats.moves[i];
        let p_before = if rec.side == "White" {
            wp_white[i]
        } else {
            wp_black[i]
        };
        let p_after = if i + 1 < n {
            if rec.side == "White" {
                wp_white[i + 1]
            } else {
                wp_black[i + 1]
            }
        } else {
            if rec.side == "White" {
                p_term_white
            } else {
                p_term_black
            }
        };
        let delta = (p_before - p_after).max(0.0);

        let before_hfen = if i == 0 {
            &starting_hfen
        } else {
            &stats.moves[i - 1].hfen_after
        };
        let after_hfen = &rec.hfen_after;

        if rec.side == "White" {
            w_losses.push(delta);
            let mat_before = get_material_value(before_hfen, "White");
            let mat_after = get_material_value(after_hfen, "White");
            if mat_after < mat_before {
                w_sac_attempted = true;
                if delta <= 0.02 && p_before < 0.9 {
                    w_brilliant_count += 1;
                }
            }
        } else {
            b_losses.push(delta);
            let mat_before = get_material_value(before_hfen, "Black");
            let mat_after = get_material_value(after_hfen, "Black");
            if mat_after < mat_before {
                b_sac_attempted = true;
                if delta <= 0.02 && p_before < 0.9 {
                    b_brilliant_count += 1;
                }
            }
        }
    }

    let w_sound = if w_losses.is_empty() {
        1.0
    } else {
        1.0 - w_losses.iter().sum::<f64>() / w_losses.len() as f64
    };
    let b_sound = if b_losses.is_empty() {
        1.0
    } else {
        1.0 - b_losses.iter().sum::<f64>() / b_losses.len() as f64
    };

    let w_brill = if w_sac_attempted {
        (w_sound + 0.1 * w_brilliant_count as f64).min(1.0)
    } else {
        w_sound
    };
    let b_brill = if b_sac_attempted {
        (b_sound + 0.1 * b_brilliant_count as f64).min(1.0)
    } else {
        b_sound
    };

    let mut w_eff = 1.0;
    let mut b_eff = 1.0;

    if let Some(w_t0) = wp_white.iter().position(|&wp| wp >= 0.85) {
        let plies_to_end = n - w_t0;
        w_eff = (1.0 - plies_to_end as f64 / 100.0).max(0.0);
        if wp_white[w_t0..].iter().any(|&wp| wp < 0.5) {
            w_eff *= 0.5;
        }
    }
    if let Some(b_t0) = wp_black.iter().position(|&wp| wp >= 0.85) {
        let plies_to_end = n - b_t0;
        b_eff = (1.0 - plies_to_end as f64 / 100.0).max(0.0);
        if wp_black[b_t0..].iter().any(|&wp| wp < 0.5) {
            b_eff *= 0.5;
        }
    }

    let mut w_res = 1.0;
    let mut b_res = 1.0;

    let w_lost_positions: Vec<usize> = wp_white
        .iter()
        .enumerate()
        .filter(|&(_, &wp)| wp < 0.15)
        .map(|(idx, _)| idx)
        .collect();
    if !w_lost_positions.is_empty() {
        let mut lost_losses = Vec::new();
        for &idx in &w_lost_positions {
            if idx % 2 == 0 && idx / 2 < w_losses.len() {
                lost_losses.push(w_losses[idx / 2]);
            }
        }
        let mean_lost_loss = if lost_losses.is_empty() {
            0.0
        } else {
            lost_losses.iter().sum::<f64>() / lost_losses.len() as f64
        };
        w_res = (n as f64 / 150.0).min(1.0) * (1.0 - mean_lost_loss);
    }

    let b_lost_positions: Vec<usize> = wp_black
        .iter()
        .enumerate()
        .filter(|&(_, &wp)| wp < 0.15)
        .map(|(idx, _)| idx)
        .collect();
    if !b_lost_positions.is_empty() {
        let mut lost_losses = Vec::new();
        for &idx in &b_lost_positions {
            if idx % 2 != 0 && idx / 2 < b_losses.len() {
                lost_losses.push(b_losses[idx / 2]);
            }
        }
        let mean_lost_loss = if lost_losses.is_empty() {
            0.0
        } else {
            lost_losses.iter().sum::<f64>() / lost_losses.len() as f64
        };
        b_res = (n as f64 / 150.0).min(1.0) * (1.0 - mean_lost_loss);
    }

    let w_code = if p_term_white > 0.6 {
        1
    } else if p_term_white < 0.4 {
        3
    } else {
        2
    };
    let b_code = if p_term_black > 0.6 {
        1
    } else if p_term_black < 0.4 {
        3
    } else {
        2
    };

    let w_mastery = weighted_penalized_min(w_sound, w_brill, w_eff, w_res, w_code);
    let b_mastery = weighted_penalized_min(b_sound, b_brill, b_eff, b_res, b_code);

    let w_band = if p_term_white > 0.6 {
        0.67
    } else if p_term_white < 0.4 {
        0.0
    } else {
        0.33
    };
    let b_band = if p_term_black > 0.6 {
        0.67
    } else if p_term_black < 0.4 {
        0.0
    } else {
        0.33
    };

    let refined_w = w_band + 0.33 * w_mastery;
    let refined_b = b_band + 0.33 * b_mastery;

    let v_target_w = 2.0 * refined_w - 1.0;
    let v_target_b = 2.0 * refined_b - 1.0;

    (
        MasteryMetrics {
            soundness: w_sound,
            brilliance: w_brill,
            efficiency: w_eff,
            resilience: w_res,
            mastery: w_mastery,
            refined_outcome: refined_w,
            v_target: v_target_w,
        },
        MasteryMetrics {
            soundness: b_sound,
            brilliance: b_brill,
            efficiency: b_eff,
            resilience: b_res,
            mastery: b_mastery,
            refined_outcome: refined_b,
            v_target: v_target_b,
        },
    )
}

/// Save full detailed game stats as a `.txt` file.
pub fn save_game_stats(board: &Board, stats: &GameStats, path: &str) {
    let mut out = String::with_capacity(8192);

    // ── 1. Header ──────────────────────────────────────────────────────────
    let now = chrono::Local::now();
    out.push_str("╔══════════════════════════════════════════════════════════════════╗\n");
    out.push_str("║              HYPERCHESS CUDA ENGINE — GAME RECORD               ║\n");
    out.push_str("╚══════════════════════════════════════════════════════════════════╝\n\n");

    out.push_str(&format!(
        "Date/Time  : {}\n",
        now.format("%Y-%m-%d %H:%M:%S")
    ));
    out.push_str(&format!("White      : {}\n", stats.white_engine));
    out.push_str(&format!("Black      : {}\n", stats.black_engine));

    // Selective search parameters display
    let white_mcts = stats.white_engine.to_lowercase().contains("mcts");
    let black_mcts = stats.black_engine.to_lowercase().contains("mcts");
    if white_mcts || black_mcts {
        if white_mcts && black_mcts {
            if stats.white_simulations == stats.black_simulations {
                out.push_str(&format!("Simulations: {}\n", stats.white_simulations));
            } else {
                out.push_str(&format!("White Sims : {}\n", stats.white_simulations));
                out.push_str(&format!("Black Sims : {}\n", stats.black_simulations));
            }
        } else if white_mcts {
            out.push_str(&format!("White Sims : {}\n", stats.white_simulations));
            out.push_str(&format!("Black Depth: {}\n", stats.black_depth));
        } else {
            out.push_str(&format!("White Depth: {}\n", stats.white_depth));
            out.push_str(&format!("Black Sims : {}\n", stats.black_simulations));
        }
    } else {
        if stats.white_depth == stats.black_depth {
            out.push_str(&format!("Depth      : {}\n", stats.white_depth));
        } else {
            out.push_str(&format!("White Depth: {}\n", stats.white_depth));
            out.push_str(&format!("Black Depth: {}\n", stats.black_depth));
        }
    }

    if let Some(ws) = stats.white_skill {
        out.push_str(&format!("White Skill: {} / 20\n", ws));
    }
    if let Some(bs) = stats.black_skill {
        out.push_str(&format!("Black Skill: {} / 20\n", bs));
    }
    out.push_str(&format!("Game Seed  : {}\n", stats.game_seed));
    out.push_str(&format!("Result     : {}\n", stats.result));
    out.push_str(&format!(
        "Total Moves: {} plies ({} full moves)\n",
        stats.total_moves,
        stats.total_moves.div_ceil(2)
    ));
    match &stats.gpu_name {
        Some(name) => out.push_str(&format!("GPU        : {}\n", name)),
        None => out.push_str("GPU        : not used (CPU-only build)\n"),
    }
    out.push('\n');

    // ── 2. Performance summary ─────────────────────────────────────────────
    out.push_str("── Performance ──────────────────────────────────────────────────────\n");
    let total_gpu = stats.white_moves_on_gpu + stats.black_moves_on_gpu;
    let pct_gpu = if stats.total_moves > 0 {
        total_gpu as f64 / stats.total_moves as f64 * 100.0
    } else {
        0.0
    };

    let w_moves: Vec<&MoveRecord> = stats.moves.iter().filter(|r| r.side == "White").collect();
    let b_moves: Vec<&MoveRecord> = stats.moves.iter().filter(|r| r.side == "Black").collect();

    fn timing_stats<'a>(moves: &[&'a MoveRecord]) -> Option<(f64, &'a MoveRecord, &'a MoveRecord)> {
        if moves.is_empty() {
            return None;
        }
        let mean = moves.iter().map(|r| r.think_ms).sum::<u128>() as f64 / moves.len() as f64;
        let min = moves.iter().min_by_key(|r| r.think_ms).unwrap();
        let max = moves.iter().max_by_key(|r| r.think_ms).unwrap();
        Some((mean, min, max))
    }

    out.push_str(&format!(
        "Total think time   : {:.3}s\n",
        stats.total_think_ms as f64 / 1000.0
    ));

    if let Some((mean, min, max)) = timing_stats(&stats.moves.iter().collect::<Vec<_>>()) {
        out.push_str(&format!("  Mean per ply     : {:.1}ms\n", mean));
        out.push_str(&format!(
            "  Min              : {}ms  (ply {} — {} {})\n",
            min.think_ms, min.ply, min.side, min.move_str
        ));
        out.push_str(&format!(
            "  Max              : {}ms  (ply {} — {} {})\n",
            max.think_ms, max.ply, max.side, max.move_str
        ));
    }
    out.push('\n');

    out.push_str(&format!(
        "White think time   : {:.3}s\n",
        stats.white_think_ms as f64 / 1000.0
    ));
    if let Some((mean, min, max)) = timing_stats(&w_moves) {
        out.push_str(&format!("  Mean per ply     : {:.1}ms\n", mean));
        out.push_str(&format!(
            "  Min              : {}ms  (ply {} — {})\n",
            min.think_ms, min.ply, min.move_str
        ));
        out.push_str(&format!(
            "  Max              : {}ms  (ply {} — {})\n",
            max.think_ms, max.ply, max.move_str
        ));
    }
    out.push('\n');

    out.push_str(&format!(
        "Black think time   : {:.3}s\n",
        stats.black_think_ms as f64 / 1000.0
    ));
    if let Some((mean, min, max)) = timing_stats(&b_moves) {
        out.push_str(&format!("  Mean per ply     : {:.1}ms\n", mean));
        out.push_str(&format!(
            "  Min              : {}ms  (ply {} — {})\n",
            min.think_ms, min.ply, min.move_str
        ));
        out.push_str(&format!(
            "  Max              : {}ms  (ply {} — {})\n",
            max.think_ms, max.ply, max.move_str
        ));
    }
    out.push('\n');

    if !stats.moves.is_empty() {
        let rand_mean = |recs: &[&MoveRecord]| -> Option<f64> {
            if recs.is_empty() {
                return None;
            }
            Some(recs.iter().map(|r| r.decision_rand).sum::<f64>() / recs.len() as f64)
        };
        let all_refs: Vec<&MoveRecord> = stats.moves.iter().collect();
        let mean_all = rand_mean(&all_refs).unwrap_or(f64::NAN);
        let mean_w = rand_mean(&w_moves).unwrap_or(f64::NAN);
        let mean_b = rand_mean(&b_moves).unwrap_or(f64::NAN);
        out.push_str(
            "Decision random    (uniform [0,1) drawn at commit time; expected mean ≈ 0.500)\n",
        );
        out.push_str(&format!("  Game seed        : {}\n", stats.game_seed));
        out.push_str(&format!(
            "  Mean (all plies) : {:.6}  ({} draws)\n",
            mean_all,
            all_refs.len()
        ));
        out.push_str(&format!(
            "  Mean (White)     : {:.6}  ({} draws)\n",
            mean_w,
            w_moves.len()
        ));
        out.push_str(&format!(
            "  Mean (Black)     : {:.6}  ({} draws)\n",
            mean_b,
            b_moves.len()
        ));
        out.push('\n');
    }

    out.push_str(&format!(
        "GPU-dispatched     : {} / {} plies ({:.1}%)\n",
        total_gpu, stats.total_moves, pct_gpu
    ));
    out.push_str(&format!(
        "  White on GPU     : {} plies\n",
        stats.white_moves_on_gpu
    ));
    out.push_str(&format!(
        "  Black on GPU     : {} plies\n",
        stats.black_moves_on_gpu
    ));
    out.push('\n');

    // ── 2f. Mastery Metrics ────────────────────────────────────────────────
    let (w_metrics, b_metrics) = compute_mastery(stats);
    out.push_str("── Mastery Metrics (v3.1 spec) ──────────────────────────────────────\n");
    out.push_str("White:\n");
    out.push_str(&format!(
        "  Soundness  : {:.1}%\n",
        w_metrics.soundness * 100.0
    ));
    out.push_str(&format!(
        "  Brilliance : {:.1}%\n",
        w_metrics.brilliance * 100.0
    ));
    out.push_str(&format!(
        "  Efficiency : {:.1}%\n",
        w_metrics.efficiency * 100.0
    ));
    out.push_str(&format!(
        "  Resilience : {:.1}%\n",
        w_metrics.resilience * 100.0
    ));
    out.push_str(&format!(
        "  Mastery    : {:.1}%\n",
        w_metrics.mastery * 100.0
    ));
    out.push_str(&format!(
        "  Refined Out: {:.3}\n",
        w_metrics.refined_outcome
    ));
    out.push_str(&format!("  Value Targ : {:+.3}\n", w_metrics.v_target));
    out.push('\n');
    out.push_str("Black:\n");
    out.push_str(&format!(
        "  Soundness  : {:.1}%\n",
        b_metrics.soundness * 100.0
    ));
    out.push_str(&format!(
        "  Brilliance : {:.1}%\n",
        b_metrics.brilliance * 100.0
    ));
    out.push_str(&format!(
        "  Efficiency : {:.1}%\n",
        b_metrics.efficiency * 100.0
    ));
    out.push_str(&format!(
        "  Resilience : {:.1}%\n",
        b_metrics.resilience * 100.0
    ));
    out.push_str(&format!(
        "  Mastery    : {:.1}%\n",
        b_metrics.mastery * 100.0
    ));
    out.push_str(&format!(
        "  Refined Out: {:.3}\n",
        b_metrics.refined_outcome
    ));
    out.push_str(&format!("  Value Targ : {:+.3}\n", b_metrics.v_target));
    out.push('\n');

    // ── 3. Move table ──────────────────────────────────────────────────────
    out.push_str("── Move History ─────────────────────────────────────────────────────\n");
    out.push_str(&format!(
        "{:<5} {:<6} {:<12} {:>8}cp  {:>7}ms  {:>8}  {}\n",
        "Ply", "Side", "Move", "Eval", "Think", "Rand[0,1)", "Backend"
    ));
    out.push_str(&format!("{}\n", "─".repeat(80)));

    for rec in &stats.moves {
        let eval_str = if rec.eval_score >= 0 {
            format!("+{}", rec.eval_score)
        } else {
            rec.eval_score.to_string()
        };
        out.push_str(&format!(
            "{:<5} {:<6} {:<12} {:>8}  {:>7}ms  {:>8.6}  {}\n",
            rec.ply, rec.side, rec.move_str, eval_str, rec.think_ms, rec.decision_rand, rec.backend
        ));
    }
    out.push('\n');

    // ── 4. ASCII eval sparkline ────────────────────────────────────────────
    if !stats.moves.is_empty() {
        out.push_str("── Eval Graph (White's perspective, each bar = 1 ply) ───────────────\n");
        out.push_str(&eval_sparkline(&stats.moves));
        out.push('\n');
    }

    // ── 5. Final board ─────────────────────────────────────────────────────
    out.push_str("── Final Position ───────────────────────────────────────────────────\n");
    out.push_str(&board.pretty_print());
    out.push('\n');

    // ── 6. HFEN ───────────────────────────────────────────────────────────
    out.push_str("── Final HFEN ───────────────────────────────────────────────────────\n");
    out.push_str(&board.get_hfen());
    out.push('\n');

    match fs::write(path, &out) {
        Ok(_) => println!("Stats  → {}", path),
        Err(e) => eprintln!("Error saving stats: {}", e),
    }
}

/// Save a minimal JSON game record for machine consumption.
pub fn save_json(board: &Board, stats: &GameStats, path: &str) {
    let mut json = String::with_capacity(4096);
    json.push_str("{\n");
    json.push_str(&format!("  \"white\": \"{}\",\n", esc(&stats.white_engine)));
    json.push_str(&format!("  \"black\": \"{}\",\n", esc(&stats.black_engine)));
    json.push_str(&format!(
        "  \"white_skill\": {},\n",
        stats
            .white_skill
            .map_or("null".to_string(), |s| s.to_string())
    ));
    json.push_str(&format!(
        "  \"black_skill\": {},\n",
        stats
            .black_skill
            .map_or("null".to_string(), |s| s.to_string())
    ));
    json.push_str(&format!("  \"depth\": {},\n", stats.depth));
    json.push_str(&format!("  \"white_depth\": {},\n", stats.white_depth));
    json.push_str(&format!("  \"black_depth\": {},\n", stats.black_depth));
    json.push_str(&format!(
        "  \"white_simulations\": {},\n",
        stats.white_simulations
    ));
    json.push_str(&format!(
        "  \"black_simulations\": {},\n",
        stats.black_simulations
    ));
    json.push_str(&format!("  \"result\": \"{}\",\n", esc(&stats.result)));
    json.push_str(&format!("  \"total_plies\": {},\n", stats.total_moves));
    json.push_str(&format!(
        "  \"gpu\": {},\n",
        stats
            .gpu_name
            .as_deref()
            .map(|n| format!("\"{}\"", esc(n)))
            .unwrap_or_else(|| "null".to_string())
    ));
    json.push_str(&format!("  \"game_seed\": {},\n", stats.game_seed));
    json.push_str(&format!(
        "  \"total_think_ms\": {},\n",
        stats.total_think_ms
    ));

    // Per-side and total timing stats embedded in JSON
    {
        let emit_timing = |moves: &[&MoveRecord]| -> String {
            if moves.is_empty() {
                return "null".to_string();
            }
            let n = moves.len() as f64;
            let mean = moves.iter().map(|r| r.think_ms).sum::<u128>() as f64 / n;
            let min = moves.iter().min_by_key(|r| r.think_ms).unwrap();
            let max = moves.iter().max_by_key(|r| r.think_ms).unwrap();
            format!(
                "{{\"mean_ms\":{:.1},\"min_ms\":{},\"min_ply\":{},\"max_ms\":{},\"max_ply\":{}}}",
                mean, min.think_ms, min.ply, max.think_ms, max.ply
            )
        };
        let all: Vec<&MoveRecord> = stats.moves.iter().collect();
        let w: Vec<&MoveRecord> = stats.moves.iter().filter(|r| r.side == "White").collect();
        let b: Vec<&MoveRecord> = stats.moves.iter().filter(|r| r.side == "Black").collect();
        json.push_str(&format!("  \"timing_total\": {},\n", emit_timing(&all)));
        json.push_str(&format!("  \"timing_white\": {},\n", emit_timing(&w)));
        json.push_str(&format!("  \"timing_black\": {},\n", emit_timing(&b)));

        // Mean decision_rand across all plies (expected ≈ 0.5 for a fair RNG)
        let mean_rand = if stats.moves.is_empty() {
            0.0
        } else {
            stats.moves.iter().map(|r| r.decision_rand).sum::<f64>() / stats.moves.len() as f64
        };
        json.push_str(&format!("  \"mean_decision_rand\": {:.6},\n", mean_rand));
    }

    json.push_str(&format!(
        "  \"gpu_dispatched_plies\": {},\n",
        stats.white_moves_on_gpu + stats.black_moves_on_gpu
    ));

    // Embed computed Mastery metrics (v3.1) in JSON
    let (w_metrics, b_metrics) = compute_mastery(stats);
    json.push_str("  \"white_metrics\": {\n");
    json.push_str(&format!("    \"soundness\": {:.6},\n", w_metrics.soundness));
    json.push_str(&format!(
        "    \"brilliance\": {:.6},\n",
        w_metrics.brilliance
    ));
    json.push_str(&format!(
        "    \"efficiency\": {:.6},\n",
        w_metrics.efficiency
    ));
    json.push_str(&format!(
        "    \"resilience\": {:.6},\n",
        w_metrics.resilience
    ));
    json.push_str(&format!("    \"mastery\": {:.6},\n", w_metrics.mastery));
    json.push_str(&format!(
        "    \"refined_outcome\": {:.6},\n",
        w_metrics.refined_outcome
    ));
    json.push_str(&format!("    \"v_target\": {:.6}\n", w_metrics.v_target));
    json.push_str("  },\n");

    json.push_str("  \"black_metrics\": {\n");
    json.push_str(&format!("    \"soundness\": {:.6},\n", b_metrics.soundness));
    json.push_str(&format!(
        "    \"brilliance\": {:.6},\n",
        b_metrics.brilliance
    ));
    json.push_str(&format!(
        "    \"efficiency\": {:.6},\n",
        b_metrics.efficiency
    ));
    json.push_str(&format!(
        "    \"resilience\": {:.6},\n",
        b_metrics.resilience
    ));
    json.push_str(&format!("    \"mastery\": {:.6},\n", b_metrics.mastery));
    json.push_str(&format!(
        "    \"refined_outcome\": {:.6},\n",
        b_metrics.refined_outcome
    ));
    json.push_str(&format!("    \"v_target\": {:.6}\n", b_metrics.v_target));
    json.push_str("  },\n");

    json.push_str(&format!(
        "  \"final_hfen\": \"{}\",\n",
        esc(&board.get_hfen())
    ));
    json.push_str("  \"moves\": [\n");

    for (i, rec) in stats.moves.iter().enumerate() {
        let comma = if i + 1 < stats.moves.len() { "," } else { "" };
        json.push_str(&format!(
            "    {{\"ply\":{},\"side\":\"{}\",\"move\":\"{}\",\"hpgn_i\":\"{}\",\"eval\":{},\"think_ms\":{},\"decision_rand\":{:.6},\"backend\":\"{}\",\"hfen\":\"{}\"}}{}\n",
            rec.ply, rec.side, esc(&rec.move_str), esc(&rec.hpgn_i), rec.eval_score,
            rec.think_ms, rec.decision_rand, rec.backend, esc(&rec.hfen_after), comma
        ));
    }

    json.push_str("  ]\n}\n");

    match fs::write(path, &json) {
        Ok(_) => println!("JSON   → {}", path),
        Err(e) => eprintln!("Error saving JSON: {}", e),
    }
}

// ── Internal helpers ──────────────────────────────────────────────────────────

/// Escape a string for embedding in the hand-built JSON emitted by `save_json`.
fn esc(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

/// Generate a two-line ASCII eval sparkline over the game.
fn eval_sparkline(moves: &[MoveRecord]) -> String {
    if moves.is_empty() {
        return String::new();
    }

    // Clamp eval to ±800 cp for display; scale to height 8
    let scale = 800i32;
    let bars = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

    let mut top_line = String::new(); // positive half
    let mut bot_line = String::new(); // negative half

    for rec in moves {
        let e = rec.eval_score.clamp(-scale, scale);
        let norm = (e + scale) as f32 / (2 * scale) as f32; // 0..1
        let idx = (norm * (bars.len() as f32 - 1.0)).round() as usize;
        let bar = bars[idx.min(bars.len() - 1)];
        // top shows White-positive moves (bar high), bot shows Black-positive
        top_line.push(if e >= 0 { bar } else { ' ' });
        bot_line.push(if e < 0 { bar } else { ' ' });
    }

    format!(
        "  +800cp ┤{}\n  -800cp ┤{}\n           {}   (W advantage = upper, B advantage = lower)\n",
        top_line,
        bot_line,
        "^".repeat(moves.len().min(60))
    )
}

// ── NNUE plain training format ────────────────────────────────────────────────

/// Append NNUE training data to a `.plain` file.
///
/// Format (one record per half-move, terminated by `e`):
/// ```text
/// fen <fen>
/// move <uci_move>
/// score <eval_cp_from_white_perspective>
/// ply <halfmove_number>
/// result <1/-1/0>
/// e
/// ```
/// `result`: 1 = White wins, -1 = Black wins, 0 = draw.
pub fn append_nnue_plain(stats: &GameStats, path: &str) {
    use std::io::Write;

    let result_int: i8 = match stats.result.as_str() {
        r if r.contains("White wins") => 1,
        r if r.contains("Black wins") => -1,
        _ => 0,
    };

    let mut buf = Vec::with_capacity(stats.moves.len() * 120);

    for rec in &stats.moves {
        // Score from White's perspective (MoveRecord.eval_score is already absolute)
        writeln!(buf, "fen {}", rec.hfen_after).ok();
        writeln!(buf, "move {}", rec.move_str).ok();
        writeln!(buf, "score {}", rec.eval_score).ok();
        writeln!(buf, "ply {}", rec.ply).ok();
        writeln!(buf, "result {}", result_int).ok();
        writeln!(buf, "e").ok();
    }

    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .expect("cannot open nnue plain file");
    file.write_all(&buf).ok();
    println!("Plain  → {} ({} positions)", path, stats.moves.len());
}

/// Legacy wrapper — kept so render.rs / old call sites still compile.
#[allow(dead_code)]
pub fn save_txt(
    board: &Board,
    move_history: &[(String, String)],
    result: &str,
    white_name: &str,
    black_name: &str,
    path: &str,
) {
    let mut content = String::new();
    content.push_str("=== HyperChess Game Record ===\n");
    content.push_str(&format!("White: {}\n", white_name));
    content.push_str(&format!("Black: {}\n", black_name));
    content.push_str(&format!("Result: {}\n", result));
    content.push_str(&format!("Moves: {}\n\n", move_history.len()));
    content.push_str("Move History:\n");
    for (i, (move_str, hfen)) in move_history.iter().enumerate() {
        let move_num = i / 2 + 1;
        let side = if i % 2 == 0 { "W" } else { "B" };
        content.push_str(&format!("{}. {} {} | {}\n", move_num, side, move_str, hfen));
    }
    content.push_str(&format!("\nFinal Position:\n{}\n", board.pretty_print()));
    content.push_str(&format!("HFEN: {}\n", board.get_hfen()));
    match fs::write(path, &content) {
        Ok(_) => println!("TXT    → {}", path),
        Err(e) => eprintln!("Error saving TXT: {}", e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// save_json is hand-rolled string formatting — assert the output is real
    /// JSON and that each move object carries its HPGN-I string verbatim
    /// (identity-prefixed and plain-UCI-fallback forms alike).
    #[test]
    fn test_save_json_emits_hpgn_i_per_move() {
        let mut stats = GameStats::new(
            "test_white",
            "test_black",
            4,
            4,
            100,
            100,
            None,
            None,
            None,
            1234,
        );
        stats.result = "White wins".to_string();
        stats.total_moves = 2;

        stats.push_move(MoveRecord {
            ply: 1,
            side: "White",
            move_str: "f3f4".to_string(),
            hpgn_i: "R:f3f4".to_string(),
            hfen_after: Board::start_pos().get_hfen(),
            eval_score: 50,
            think_ms: 10,
            root_moves_count: 10,
            backend: "CPU".to_string(),
            decision_rand: 0.5,
        });
        // No identity tracked at the source square → plain UCI fallback.
        stats.push_move(MoveRecord {
            ply: 2,
            side: "Black",
            move_str: "f10f9".to_string(),
            hpgn_i: "f10f9".to_string(),
            hfen_after: Board::start_pos().get_hfen(),
            eval_score: -25,
            think_ms: 12,
            root_moves_count: 8,
            backend: "CPU".to_string(),
            decision_rand: 0.25,
        });

        let path = std::env::temp_dir().join(format!(
            "hyperchess_export_hpgn_i_test_{}.json",
            std::process::id()
        ));
        let path_str = path.to_str().unwrap();

        save_json(&Board::start_pos(), &stats, path_str);

        let content = std::fs::read_to_string(&path).unwrap();
        let parsed: serde_json::Value =
            serde_json::from_str(&content).expect("save_json output must be valid JSON");

        let moves = parsed["moves"]
            .as_array()
            .expect("moves must be a JSON array");
        assert_eq!(moves.len(), 2);
        assert_eq!(moves[0]["hpgn_i"], "R:f3f4");
        assert_eq!(moves[0]["move"], "f3f4");
        assert_eq!(moves[1]["hpgn_i"], "f10f9");
        assert_eq!(moves[1]["move"], "f10f9");

        std::fs::remove_file(&path).unwrap();
    }
}
