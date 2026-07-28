// SPDX-License-Identifier: GPL-3.0-or-later
// HyperChess Core — hyperchess-rules
// File: crates/hyperchess-rules/src/io/hsan_parse.rs
// Version: 1.0.0
// Copyright (c) 2026 HyperChess Developer Team

//! HSAN parser and HSAN-to-HyperMove converter.

use crate::board::Board;
use crate::core::piece_move::HyperMove;
use crate::core::sq::SQ;
use crate::core::PieceType;

/// Parsed HSAN move components.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HsanMove {
    /// Piece character (K, Q, R, B, N) or None for pawn.
    pub piece: Option<char>,
    /// Source file disambiguation (0-11 for a-l).
    pub src_file: Option<u8>,
    /// Source rank disambiguation (0-11 for ranks 1-12).
    pub src_rank: Option<u8>,
    /// Destination file (0-11).
    pub dst_file: u8,
    /// Destination rank (0-11).
    pub dst_rank: u8,
    /// Capture marker.
    pub is_capture: bool,
    /// Promotion piece (Q, R, B, N, E, H) or None.
    pub promotion: Option<char>,
    /// Check or checkmate marker.
    pub check_marker: CheckMarker,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// The trailing annotation on a HSAN move.
pub enum CheckMarker {
    /// No marker.
    None,
    /// Trailing `+`.
    Check,
    /// Trailing `#`.
    Checkmate,
}

/// Parse HSAN move string (e.g., "e4", "Nf3", "Qxe5", "a8=Q+", "O-O").
pub fn parse_hsan(hsan: &str) -> Result<HsanMove, String> {
    let s = hsan.trim();
    if s.is_empty() {
        return Err("Empty HSAN move".to_string());
    }

    // Handle castling first
    if s == "O-O" || s == "0-0" {
        return Ok(HsanMove {
            piece: Some('K'),
            src_file: None,
            src_rank: None,
            dst_file: 0,
            dst_rank: 0,
            is_capture: false,
            promotion: None,
            check_marker: CheckMarker::None,
        });
    }
    if s == "O-O-O" || s == "0-0-0" {
        return Ok(HsanMove {
            piece: Some('K'),
            src_file: None,
            src_rank: None,
            dst_file: 0,
            dst_rank: 0,
            is_capture: false,
            promotion: None,
            check_marker: CheckMarker::None,
        });
    }

    // Remove trailing check/mate markers first
    let (s, check_marker) = if let Some(stripped) = s.strip_suffix('#') {
        (stripped, CheckMarker::Checkmate)
    } else if let Some(stripped) = s.strip_suffix('+') {
        (stripped, CheckMarker::Check)
    } else {
        (s, CheckMarker::None)
    };

    let mut chars = s.chars().peekable();

    // Parse piece symbol (K, Q, R, B, N) or None for pawn
    let piece = if let Some(&c) = chars.peek() {
        if "KQRBN".contains(c) {
            chars.next();
            Some(c)
        } else {
            None
        }
    } else {
        None
    };

    // Collect remaining characters
    let remaining: String = chars.collect();

    // Parse promotion first (we know it ends the move before check/mate)
    let (remaining, promotion) = if let Some(pos) = remaining.find('=') {
        let (before_promo, after_eq) = remaining.split_at(pos);
        let promo_char = after_eq.chars().nth(1);
        (before_promo.to_string(), promo_char)
    } else {
        (remaining, None)
    };

    // Parse capture marker
    let (remaining, is_capture) = if remaining.contains('x') {
        (remaining.replace('x', ""), true)
    } else {
        (remaining, false)
    };

    // Now parse what's left: [disamb-file] [disamb-rank] dest-file dest-rank
    // The last two characters are the destination; anything before is disambiguation
    if remaining.len() < 2 {
        return Err("Missing destination square".to_string());
    }

    let disamb_and_dest: Vec<char> = remaining.chars().collect();
    let dest_file_char = disamb_and_dest[disamb_and_dest.len() - 2];
    let dest_rank_char = disamb_and_dest[disamb_and_dest.len() - 1];

    // Parse destination
    if !('a'..='l').contains(&dest_file_char) {
        return Err(format!("Invalid destination file: {}", dest_file_char));
    }
    let dst_file = (dest_file_char as u8) - b'a';

    if !dest_rank_char.is_ascii_digit() {
        return Err(format!("Invalid destination rank: {}", dest_rank_char));
    }
    let rank_num: u32 = dest_rank_char.to_digit(10).ok_or("Invalid rank")?;
    if rank_num == 0 || rank_num > 12 {
        return Err(format!("Destination rank out of bounds: {}", rank_num));
    }
    let dst_rank = (rank_num as u8) - 1;

    // Parse disambiguation (if any)
    let mut src_file = None;
    let mut src_rank = None;
    if disamb_and_dest.len() > 2 {
        let disamb: String = disamb_and_dest[..disamb_and_dest.len() - 2]
            .iter()
            .collect();
        for c in disamb.chars() {
            if ('a'..='l').contains(&c) {
                src_file = Some((c as u8) - b'a');
            } else if c.is_ascii_digit() {
                if let Some(r) = c.to_digit(10) {
                    src_rank = Some((r as u8) - 1);
                }
            }
        }
    }

    Ok(HsanMove {
        piece,
        src_file,
        src_rank,
        dst_file,
        dst_rank,
        is_capture,
        promotion,
        check_marker,
    })
}

/// Convert parsed HSAN move to HyperMove by matching against legal moves.
pub fn hsan_to_hypermove(board: &Board, hsan_move: &HsanMove) -> Result<HyperMove, String> {
    // Handle castling
    if hsan_move.piece == Some('K') && hsan_move.src_file.is_none() && hsan_move.src_rank.is_none()
    {
        let legal = board.generate_moves();
        for m in legal.iter() {
            if m.is_king_castle() {
                return Ok(*m);
            }
        }
        for m in legal.iter() {
            if m.is_queen_castle() {
                return Ok(*m);
            }
        }
        return Err("No legal castling move found".to_string());
    }

    let dst = SQ::make(hsan_move.dst_file, hsan_move.dst_rank);
    let legal = board.generate_moves();

    let candidates: Vec<_> = legal
        .iter()
        .filter(|m| m.get_dest() == dst)
        .copied()
        .collect();

    if candidates.is_empty() {
        return Err(format!(
            "No legal move to square ({}, {})",
            (hsan_move.dst_file + b'a') as char,
            hsan_move.dst_rank + 1
        ));
    }

    if candidates.len() == 1 {
        return Ok(candidates[0]);
    }

    let filtered: Vec<_> = candidates
        .iter()
        .filter(|m| {
            if let Some(promo_char) = hsan_move.promotion {
                if !m.is_promo() {
                    return false;
                }
                let promo_type = PieceType::from_char(promo_char);
                if promo_type != Some(m.promo_piece()) {
                    return false;
                }
            } else if m.is_promo() {
                return false;
            }

            if hsan_move.is_capture != m.is_capture() {
                return false;
            }

            if let Some(src_file) = hsan_move.src_file {
                if m.get_src().0 % 12 != src_file {
                    return false;
                }
            }

            if let Some(src_rank) = hsan_move.src_rank {
                if m.get_src().0 / 12 != src_rank {
                    return false;
                }
            }

            true
        })
        .copied()
        .collect();

    match filtered.len() {
        1 => Ok(filtered[0]),
        0 => Err("No legal move matches the HSAN disambiguators".to_string()),
        _ => Err(format!(
            "Ambiguous HSAN move: {} legal moves match",
            filtered.len()
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_hsan_quiet_pawn() {
        let hsan = parse_hsan("e4").unwrap();
        assert_eq!(hsan.piece, None);
        assert_eq!(hsan.dst_file, 4);
        assert_eq!(hsan.dst_rank, 3);
        assert!(!hsan.is_capture);
    }

    #[test]
    fn test_parse_hsan_knight() {
        let hsan = parse_hsan("Nf3").unwrap();
        assert_eq!(hsan.piece, Some('N'));
        assert_eq!(hsan.dst_file, 5);
        assert_eq!(hsan.dst_rank, 2);
    }

    #[test]
    fn test_parse_hsan_capture() {
        let hsan = parse_hsan("Qxe5").unwrap();
        assert_eq!(hsan.piece, Some('Q'));
        assert!(hsan.is_capture);
    }

    #[test]
    fn test_parse_hsan_promotion() {
        let hsan = parse_hsan("a8=Q").unwrap();
        assert_eq!(hsan.piece, None);
        assert_eq!(hsan.promotion, Some('Q'));
    }

    #[test]
    fn test_parse_hsan_castling() {
        assert!(parse_hsan("O-O").is_ok());
        assert!(parse_hsan("O-O-O").is_ok());
    }

    #[test]
    fn test_parse_hsan_check() {
        let hsan = parse_hsan("Nf3+").unwrap();
        assert_eq!(hsan.check_marker, CheckMarker::Check);
    }

    #[test]
    fn test_parse_hsan_checkmate() {
        let hsan = parse_hsan("Qh5#").unwrap();
        assert_eq!(hsan.check_marker, CheckMarker::Checkmate);
    }
}
