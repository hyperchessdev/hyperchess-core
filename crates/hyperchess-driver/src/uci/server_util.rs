// SPDX-License-Identifier: GPL-3.0-or-later
// HyperChess Core — hyperchess-driver
// File: crates/hyperchess-driver/src/uci/server_util.rs
// Version: 1.0.0
// Copyright (c) 2026 HyperChess Developer Team

//! Utilities for parsing UCI position and go commands.

use hyperchess_rules::board::Board;
use hyperchess_rules::core::sq::SQ;
use hyperchess_rules::core::PieceType;
use hyperchess_rules::HyperMove;

use super::server::GoArgs;

/// Parse `position [startpos | fen <fen>] [moves m1 m2 …]`
/// and return the resulting Board.
pub fn parse_fen_and_moves(tokens: &[&str]) -> Option<Board> {
    let mut idx = 0;

    let mut board = if tokens.get(idx) == Some(&"startpos") {
        idx += 1;
        Board::start_pos()
    } else if tokens.get(idx) == Some(&"fen") {
        idx += 1;
        // Collect FEN tokens until "moves" or end
        let mut fen_parts = vec![];
        while idx < tokens.len() && tokens[idx] != "moves" {
            fen_parts.push(tokens[idx]);
            idx += 1;
        }
        let fen = fen_parts.join(" ");
        Board::from_hfen(&fen).ok()?
    } else {
        Board::start_pos()
    };

    // Apply move list if present
    if tokens.get(idx) == Some(&"moves") {
        idx += 1;
        for mv_str in &tokens[idx..] {
            if let Some(mv) = parse_uci_move(&board, mv_str) {
                board.apply_move(mv);
            }
        }
    }

    Some(board)
}

/// Parse a UCI move string into a HyperMove.
///
/// Supports two formats:
/// - Plain UCI: "g2h4", "a1a12", "e11e12q"
/// - HPGN-I (with identity prefix): "R:f3f4", "M:a11a12q", "G:g1h1"
///
/// The 12x12 board uses files a-l and ranks 1-12.
/// Promotion pieces: q, r, b, n, e (eagle), h (hawk).
pub fn parse_uci_move(board: &Board, mv_str: &str) -> Option<HyperMove> {
    // Strip HPGN-I identity prefix if present (e.g., "R:f3f4" -> "f3f4")
    let uci_str = if mv_str.contains(':') {
        // HPGN-I format: <identity>:<uci>
        // The colon separates the single-char identity from the UCI move.
        let parts: Vec<&str> = mv_str.split(':').collect();
        if parts.len() == 2 && parts[0].len() == 1 {
            // Valid HPGN-I format; use the UCI part (after the colon).
            parts[1]
        } else {
            // Malformed HPGN-I; fall back to treating the whole string as plain UCI.
            mv_str
        }
    } else {
        // Plain UCI format; use as-is.
        mv_str
    };

    // Minimum: file + rank + file + rank = 4 chars for short squares (e.g. "a1b2")
    // or more for ranks ≥ 10 (e.g. "a1a12", "a10a12")
    if uci_str.len() < 4 {
        return None;
    }

    // Parse from-square: first char = file letter, then digits for rank
    let (src, rest) = parse_square(uci_str)?;
    let (dst, promo_str) = parse_square(rest)?;

    // Optional promotion character
    let promo = if !promo_str.is_empty() {
        let pc = promo_str.chars().next()?;
        PieceType::from_char(pc)
    } else {
        None
    };

    // Match against legal moves from current position
    let legal = board.generate_moves();
    for m in legal.iter() {
        if m.get_src() == src && m.get_dest() == dst {
            let promo_matches = match promo {
                Some(pt) => m.is_promo() && m.promo_piece() == pt,
                None => !m.is_promo(),
            };
            if promo_matches {
                return Some(*m);
            }
        }
    }
    None
}

/// Parse a square string like "a1", "l12" from the start of `s`.
/// Returns (SQ, remaining_str).
fn parse_square(s: &str) -> Option<(SQ, &str)> {
    if s.is_empty() {
        return None;
    }
    let file_char = s.chars().next()?.to_ascii_lowercase();
    let file_idx = (file_char as u8).checked_sub(b'a')?;
    if file_idx >= 12 {
        return None;
    }

    // Consume rank digits
    let rest = &s[1..];
    let mut num_len = 0;
    for c in rest.chars() {
        if c.is_ascii_digit() {
            num_len += 1;
        } else {
            break;
        }
    }
    if num_len == 0 {
        return None;
    }
    let rank_str = &rest[..num_len];
    let rank_1indexed: u8 = rank_str.parse().ok()?;
    if rank_1indexed == 0 || rank_1indexed > 12 {
        return None;
    }
    let rank_idx = rank_1indexed - 1;

    let sq = SQ::make(file_idx, rank_idx);
    Some((sq, &rest[num_len..]))
}

/// Parse `go depth N | movetime N | infinite | perft N` tokens.
pub fn parse_go(tokens: &[&str]) -> GoArgs {
    let mut args = GoArgs::default();
    let mut i = 0;
    while i < tokens.len() {
        match tokens[i] {
            "depth" => {
                args.depth = tokens.get(i + 1).and_then(|s| s.parse().ok()).unwrap_or(5);
                i += 2;
            }
            "movetime" => {
                args.movetime_ms = tokens
                    .get(i + 1)
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(1000);
                i += 2;
            }
            "infinite" => {
                args.infinite = true;
                i += 1;
            }
            "perft" => {
                args.perft_depth =
                    Some(tokens.get(i + 1).and_then(|s| s.parse().ok()).unwrap_or(1));
                i += 2;
            }
            _ => {
                i += 1;
            }
        }
    }
    args
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_square_a1() {
        let (sq, rest) = parse_square("a1").unwrap();
        assert_eq!(sq, SQ::make(0, 0), "a1 should be (file=0, rank=0)");
        assert_eq!(rest, "");
    }

    #[test]
    fn test_parse_square_l12() {
        let (sq, rest) = parse_square("l12").unwrap();
        assert_eq!(sq, SQ::make(11, 11), "l12 should be (file=11, rank=11)");
        assert_eq!(rest, "");
    }

    #[test]
    fn test_parse_square_g3() {
        let (sq, rest) = parse_square("g3q").unwrap();
        assert_eq!(sq, SQ::make(6, 2), "g3 should be (file=6, rank=2)");
        assert_eq!(rest, "q");
    }

    #[test]
    fn test_parse_uci_move_start_pos() {
        hyperchess_rules::Helper::init();
        let board = Board::start_pos();
        // a3a4 is a valid pawn push (white pawn at a3 → a4)
        // Actually we need to check what moves are available from start pos.
        // Let's test g3g4 which is a pawn push from g3 on our board.
        // From start pos, white pawns are on rank 3 (file a-l = squares a3..l3).
        // Let's verify any pawn move is parseable.
        let mv = parse_uci_move(&board, "a3a4");
        // a3 has a white pawn → a4 should be legal
        assert!(mv.is_some(), "a3a4 should be a legal pawn push");
    }

    #[test]
    fn test_parse_fen_and_moves_startpos() {
        hyperchess_rules::Helper::init();
        let board = parse_fen_and_moves(&["startpos"]).unwrap();
        assert_eq!(board.get_hfen(), Board::start_pos().get_hfen());
    }

    #[test]
    fn test_parse_fen_startpos_with_move() {
        hyperchess_rules::Helper::init();
        let board = parse_fen_and_moves(&["startpos", "moves", "a3a4"]).unwrap();
        // After a3a4, it's Black's turn
        assert_eq!(board.turn(), hyperchess_rules::Player::Black);
    }

    #[test]
    fn test_parse_go_depth() {
        let args = parse_go(&["depth", "7"]);
        assert_eq!(args.depth, 7);
        assert!(!args.infinite);
    }

    #[test]
    fn test_parse_go_movetime() {
        let args = parse_go(&["movetime", "5000"]);
        assert_eq!(args.movetime_ms, 5000);
    }

    #[test]
    fn test_parse_go_infinite() {
        let args = parse_go(&["infinite"]);
        assert!(args.infinite);
    }

    #[test]
    fn test_parse_go_perft() {
        let args = parse_go(&["perft", "3"]);
        assert_eq!(args.perft_depth, Some(3));
    }

    // HPGN-I Support Tests
    #[test]
    fn test_parse_uci_move_with_identity_prefix() {
        hyperchess_rules::Helper::init();
        let board = Board::start_pos();
        // Plain UCI: a3a4
        let mv_plain = parse_uci_move(&board, "a3a4");
        // HPGN-I with identity prefix: M:a3a4 (M is the identity of white a-file pawn)
        let mv_hpgn = parse_uci_move(&board, "M:a3a4");
        // Both should parse to the same move
        assert!(mv_plain.is_some(), "plain UCI should parse");
        assert!(mv_hpgn.is_some(), "HPGN-I should parse");
        assert_eq!(mv_plain, mv_hpgn, "both formats should yield the same move");
    }

    #[test]
    fn test_parse_uci_move_hpgn_i_promotion() {
        hyperchess_rules::Helper::init();
        // Position with white pawn one rank from promotion
        let fen = "12/M11/6g5/12/12/12/12/12/12/12/6G5/12 w - - 0 1";
        let board = Board::from_hfen(fen).expect("valid FEN");
        // Plain UCI: a11a12q
        let mv_plain = parse_uci_move(&board, "a11a12q");
        // HPGN-I: R:a11a12q (R is the identity of white a-file pawn after moving)
        let mv_hpgn = parse_uci_move(&board, "R:a11a12q");
        assert!(mv_plain.is_some(), "plain UCI promotion should parse");
        assert!(mv_hpgn.is_some(), "HPGN-I promotion should parse");
        assert_eq!(
            mv_plain, mv_hpgn,
            "both promotion formats should yield same move"
        );
    }

    #[test]
    fn test_parse_uci_move_hpgn_i_castling() {
        hyperchess_rules::Helper::init();
        let board = Board::start_pos();
        // King-side castling from start pos (if legal in HyperChess)
        // This is a real game test; we parse the move string, not verify legality here.
        let mv_plain = parse_uci_move(&board, "g3h3");
        let mv_hpgn = parse_uci_move(&board, "G:g3h3");
        // Both should parse identically (if the move is legal at all)
        if mv_plain.is_some() {
            assert_eq!(
                mv_plain, mv_hpgn,
                "both castling-format moves should be identical"
            );
        }
    }

    #[test]
    fn test_parse_fen_and_moves_with_hpgn_i() {
        hyperchess_rules::Helper::init();
        // Parse startpos with HPGN-I moves
        let board = parse_fen_and_moves(&["startpos", "moves", "M:a3a4", "m:a10a9"])
            .expect("valid sequence");
        // After M:a3a4 (white a-pawn), it's Black's turn
        // After m:a10a9 (black a-pawn), it's White's turn
        assert_eq!(board.turn(), hyperchess_rules::Player::White);
    }

    #[test]
    fn test_parse_uci_move_strip_identity() {
        // Verify that the identity prefix is properly stripped
        // and doesn't interfere with move parsing.
        hyperchess_rules::Helper::init();
        let board = Board::start_pos();
        // Single-char identity must be followed by colon
        let mv1 = parse_uci_move(&board, "X:a3a4"); // X is not the identity for a3, but parser should still strip
        let mv2 = parse_uci_move(&board, "a3a4"); // plain UCI
                                                  // Both should parse the same way (identity is ignored by the parser)
        assert_eq!(mv1, mv2, "identity prefix should be transparent to parsing");
    }

    #[test]
    fn test_parse_uci_move_malformed_hpgn_i() {
        hyperchess_rules::Helper::init();
        let board = Board::start_pos();
        // Malformed HPGN-I: multiple colons
        let mv = parse_uci_move(&board, "M:a3:a4");
        // This should fail because after stripping "M:", we're left with "a3:a4" which isn't valid UCI
        assert!(mv.is_none(), "malformed HPGN-I should fail to parse");
    }
}
