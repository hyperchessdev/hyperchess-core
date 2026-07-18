//! Classical ray-based sliding attack generation for Rook, Bishop, and Queen.
//!
//! Since we use a 3×u64 bitboard, magic bitboards are not feasible.
//! Instead we use simple ray-walking.

use crate::core::bitboard::BitBoard;
use crate::core::masks::*;
use crate::core::sq::SQ;

/// Generates rook attacks from `sq` given `occupied` pieces.
#[inline]
pub fn rook_attacks(sq: SQ, occupied: BitBoard) -> BitBoard {
    let mut attacks = BitBoard::EMPTY;
    for &dir in ROOK_DIRS.iter() {
        attacks |= ray_attacks(sq, dir, occupied);
    }
    attacks
}

/// Generates bishop attacks from `sq` given `occupied` pieces.
#[inline]
pub fn bishop_attacks(sq: SQ, occupied: BitBoard) -> BitBoard {
    let mut attacks = BitBoard::EMPTY;
    for &dir in BISHOP_DIRS.iter() {
        attacks |= ray_attacks(sq, dir, occupied);
    }
    attacks
}

/// Generates queen attacks (rook + bishop).
#[inline]
pub fn queen_attacks(sq: SQ, occupied: BitBoard) -> BitBoard {
    rook_attacks(sq, occupied) | bishop_attacks(sq, occupied)
}

/// Walks a ray from `sq` in `direction`, adding each square to the result.
/// Stops when hitting an occupied square (includes that square) or going off-board.
fn ray_attacks(sq: SQ, direction: i16, occupied: BitBoard) -> BitBoard {
    let mut attacks = BitBoard::EMPTY;
    let mut current = sq.0 as i16;
    let _start_file = sq.file_idx() as i16;

    loop {
        let next = current + direction;
        if !(0..144).contains(&next) {
            break;
        }
        let next_file = next % 12;
        let curr_file = current % 12;

        // Check for file wrapping (direction should cause at most 1 file change per step)
        let file_diff = (next_file - curr_file).abs();
        if file_diff > 1 {
            break;
        }

        let next_sq = SQ(next as u8);
        attacks.set_bit(next_sq);

        // Stop at first occupied square (include it for potential capture)
        if occupied.test_bit(next_sq) {
            break;
        }

        current = next;
    }
    attacks
}

/// Returns pseudo-attacks for a sliding piece (as if the board were empty).
/// Useful for determining if two squares are on the same ray.
pub fn rook_pseudo_attacks(sq: SQ) -> BitBoard {
    rook_attacks(sq, BitBoard::EMPTY)
}

pub fn bishop_pseudo_attacks(sq: SQ) -> BitBoard {
    bishop_attacks(sq, BitBoard::EMPTY)
}

pub fn queen_pseudo_attacks(sq: SQ) -> BitBoard {
    queen_attacks(sq, BitBoard::EMPTY)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rook_attacks_center() {
        // Rook on e5 (file 4, rank 4) = sq 52, empty board
        let sq = SQ::make(4, 4);
        let attacks = rook_attacks(sq, BitBoard::EMPTY);
        // Should attack all squares on file 4 + rank 4, minus itself
        // File: 11 squares. Rank: 11 squares. Total: 22
        assert_eq!(attacks.count_bits(), 22, "Rook on e5 empty board");
    }

    #[test]
    fn test_rook_attacks_blocked() {
        let sq = SQ::make(4, 4); // e5
        let mut occupied = BitBoard::EMPTY;
        occupied.set_bit(SQ::make(4, 6)); // e7 blocks north
        occupied.set_bit(SQ::make(7, 4)); // h5 blocks east
        let attacks = rook_attacks(sq, occupied);
        // North: 4,5 and 4,6 (blocked at 6) = 2
        // South: 4,3; 4,2; 4,1; 4,0 = 4
        // East: 5,4; 6,4; 7,4 (blocked at 7) = 3
        // West: 3,4; 2,4; 1,4; 0,4 = 4
        // Total: 2 + 4 + 3 + 4 = 13
        assert_eq!(attacks.count_bits(), 13, "Rook on e5 with blockers");
    }

    #[test]
    fn test_bishop_attacks_corner() {
        // Bishop on a1 (sq 0), empty board
        let sq = SQ(0);
        let attacks = bishop_attacks(sq, BitBoard::EMPTY);
        // Only NE diagonal: b2, c3, d4, e5, f6, g7, h8, i9, j10, k11, l12 = 11
        assert_eq!(attacks.count_bits(), 11, "Bishop on a1 empty board");
    }

    #[test]
    fn test_queen_attacks() {
        let sq = SQ::make(6, 6); // g7
        let attacks = queen_attacks(sq, BitBoard::EMPTY);
        // Rook: file(11) + rank(11) = 22
        // Bishop: 4 diagonals
        let rook = rook_attacks(sq, BitBoard::EMPTY);
        let bishop = bishop_attacks(sq, BitBoard::EMPTY);
        assert_eq!(attacks, rook | bishop);
    }
}
