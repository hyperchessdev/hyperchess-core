//! Export HyperMove to HSAN (HyperChess Standard Algebraic Notation).

use crate::board::Board;
use crate::core::piece_move::HyperMove;
use crate::core::Piece;

/// Convert HyperMove to HSAN notation.
pub fn hypermove_to_hsan(board: &Board, m: HyperMove) -> String {
    // Handle castling specially
    if m.is_king_castle() {
        return "O-O".to_string();
    }
    if m.is_queen_castle() {
        return "O-O-O".to_string();
    }

    let src = m.get_src();
    let dst = m.get_dest();
    let piece = board.piece_at(src);

    let mut hsan = String::new();

    // Piece symbol (not for pawns in HSAN)
    if piece != Piece::None && piece != Piece::WhitePawn && piece != Piece::BlackPawn {
        hsan.push(piece.character_lossy());
    }

    // Destination
    hsan.push((b'a' + dst.0 % 12) as char);
    hsan.push_str(&(dst.0 / 12 + 1).to_string());

    // Capture marker (insert before destination)
    if m.is_capture() {
        if piece == Piece::WhitePawn || piece == Piece::BlackPawn {
            // For pawn captures, it's "axb3" format
            let pawn_file = (b'a' + src.0 % 12) as char;
            hsan = format!("{}x{}", pawn_file, &hsan);
        } else {
            // For piece captures, it's "Rxd1" format (insert 'x' before destination)
            hsan.insert(hsan.len().saturating_sub(2), 'x');
        }
    }

    // Promotion
    if m.is_promo() {
        hsan.push('=');
        hsan.push(
            m.promo_piece()
                .char_lower()
                .to_uppercase()
                .next()
                .unwrap_or('Q'),
        );
    }

    hsan
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hsan_export_exists() {
        // Smoke test: ensure function exists and compiles
        let _ = hypermove_to_hsan;
    }
}
