//! Eagle and Hawk attack tables — precomputed at first call, looked up thereafter.
//!
//! **Eagle** slides orthogonally, **Hawk** slides diagonally. Both are jumping
//! pieces: they can land on ANY square within 4 steps regardless of intervening
//! occupancy (the `_occupied` argument is ignored). The one-time table build
//! calls `jumping_ray` over all 144 squares so subsequent lookups are O(1).
//!
//! Design note: the guideline proposes pure (4,0)/(4,4) leapers (exactly 4
//! steps). CLAUDE.md and piece values describe "up to 4 squares" jumpers, which
//! is more interesting on a 12×12 board and consistent with this implementation.

use std::sync::OnceLock;

use crate::core::bitboard::BitBoard;
use crate::core::masks::*;
use crate::core::sq::SQ;

// ── Statics ──────────────────────────────────────────────────────────────────

static EAGLE_TABLE: OnceLock<[BitBoard; 144]> = OnceLock::new();
static HAWK_TABLE: OnceLock<[BitBoard; 144]> = OnceLock::new();

// ── Public API ────────────────────────────────────────────────────────────────

/// Returns all squares reachable by an Eagle (orthogonal jumper, max 4 steps).
///
/// `_occupied` is accepted for API compatibility with sliding-piece helpers but
/// is always ignored — Eagles jump over any pieces.
#[inline(always)]
pub fn eagle_attacks(sq: SQ, _occupied: BitBoard) -> BitBoard {
    EAGLE_TABLE.get_or_init(build_eagle)[sq.0 as usize]
}

/// Returns all squares reachable by a Hawk (diagonal jumper, max 4 steps).
#[inline(always)]
pub fn hawk_attacks(sq: SQ, _occupied: BitBoard) -> BitBoard {
    HAWK_TABLE.get_or_init(build_hawk)[sq.0 as usize]
}

// ── Table builders ────────────────────────────────────────────────────────────

fn build_eagle() -> [BitBoard; 144] {
    let mut table = [BitBoard::EMPTY; 144];
    for i in 0u8..144 {
        let mut attacks = BitBoard::EMPTY;
        for &dir in ROOK_DIRS.iter() {
            attacks |= jumping_ray(SQ(i), dir);
        }
        table[i as usize] = attacks;
    }
    table
}

fn build_hawk() -> [BitBoard; 144] {
    let mut table = [BitBoard::EMPTY; 144];
    for i in 0u8..144 {
        let mut attacks = BitBoard::EMPTY;
        for &dir in BISHOP_DIRS.iter() {
            attacks |= jumping_ray(SQ(i), dir);
        }
        table[i as usize] = attacks;
    }
    table
}

/// Walk one ray up to 4 steps, collecting every reachable square.
/// Stops at board edge or after 4 steps; never stopped by occupancy.
fn jumping_ray(sq: SQ, direction: i16) -> BitBoard {
    let mut attacks = BitBoard::EMPTY;
    let mut current = sq.0 as i16;

    for _ in 0..4 {
        let next = current + direction;
        if !(0..144).contains(&next) {
            break;
        }
        // Guard against file-wrap (direction ±1 must not cross file boundary).
        let curr_file = current % 12;
        let next_file = next % 12;
        if (next_file - curr_file).abs() > 1 {
            break;
        }
        attacks.set_bit(SQ(next as u8));
        current = next;
    }
    attacks
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Eagle ─────────────────────────────────────────────────────────────────

    #[test]
    fn eagle_max_4_squares() {
        let sq = SQ::make(0, 0); // a1
        let attacks = eagle_attacks(sq, BitBoard::EMPTY);
        assert!(attacks.test_bit(SQ::make(0, 1))); // a2 step 1
        assert!(attacks.test_bit(SQ::make(0, 2))); // a3 step 2
        assert!(attacks.test_bit(SQ::make(0, 3))); // a4 step 3
        assert!(attacks.test_bit(SQ::make(0, 4))); // a5 step 4
        assert!(!attacks.test_bit(SQ::make(0, 5))); // a6 — beyond range
    }

    /// Corner a1: north (4) + east (4) reachable, south and west are off-board.
    #[test]
    fn eagle_on_corner_attacks_8_squares() {
        let sq = SQ::make(0, 0);
        assert_eq!(eagle_attacks(sq, BitBoard::EMPTY).count_bits(), 8);
    }

    #[test]
    fn eagle_jumps_over_pieces() {
        let sq = SQ::make(0, 0);
        let mut occ = BitBoard::EMPTY;
        occ.set_bit(SQ::make(0, 1)); // blocker at a2 — ignored by Eagle
        let attacks = eagle_attacks(sq, occ);
        assert!(attacks.test_bit(SQ::make(0, 1))); // can still land on a2
        assert!(attacks.test_bit(SQ::make(0, 2))); // and a3 behind it
    }

    #[test]
    fn eagle_no_file_wrap() {
        let sq = SQ::make(0, 0); // a1 — west direction immediately exits board
        let attacks = eagle_attacks(sq, BitBoard::EMPTY);
        assert!(!attacks.test_bit(SQ::make(11, 0))); // l1 — no wrap
    }

    /// e6 (file=4, rank=5): all 4 orthogonal directions have 4 full steps.
    #[test]
    fn eagle_on_e6_attacks_16_squares() {
        let sq = SQ::make(4, 5); // e6 = SQ(64)
        assert_eq!(eagle_attacks(sq, BitBoard::EMPTY).count_bits(), 16);
    }

    // ── Hawk ──────────────────────────────────────────────────────────────────

    #[test]
    fn hawk_max_4_squares() {
        let sq = SQ::make(4, 4); // e5
        let attacks = hawk_attacks(sq, BitBoard::EMPTY);
        assert!(attacks.test_bit(SQ::make(5, 5))); // f6 step 1
        assert!(attacks.test_bit(SQ::make(6, 6))); // g7 step 2
        assert!(attacks.test_bit(SQ::make(7, 7))); // h8 step 3
        assert!(attacks.test_bit(SQ::make(8, 8))); // i9 step 4
        assert!(!attacks.test_bit(SQ::make(9, 9))); // j10 — beyond range
    }

    #[test]
    fn hawk_jumps_over_pieces() {
        let sq = SQ::make(0, 0);
        let mut occ = BitBoard::EMPTY;
        occ.set_bit(SQ::make(1, 1)); // blocker at b2 — ignored
        let attacks = hawk_attacks(sq, occ);
        assert!(attacks.test_bit(SQ::make(1, 1))); // can land on b2
        assert!(attacks.test_bit(SQ::make(2, 2))); // and c3 behind it
    }

    /// e6 (file=4, rank=5): 4 diagonal directions × 4 steps = 16 squares.
    ///
    /// The guideline's `hawk_on_e6_attacks_4_squares` assumed a pure (4,4) leaper.
    /// Our Hawk is an "up-to-4-diagonal" jumper per CLAUDE.md, giving 16 squares.
    #[test]
    fn hawk_on_e6_attacks_16_squares() {
        let sq = SQ::make(4, 5); // e6
        assert_eq!(hawk_attacks(sq, BitBoard::EMPTY).count_bits(), 16);
    }
}
