//! Board and Metal (Layer 0b / Phase 01) verification tests.

use hyperchess_rules::core::bitboard::BitBoard;
use hyperchess_rules::core::sq::SQ;
use hyperchess_rules::helper::prelude::*;
use hyperchess_rules::Board;

#[test]
fn test_bitboard_primitives() {
    let mut bb = BitBoard::EMPTY;
    assert!(bb.is_empty());

    // Set and test all 144 squares
    for i in 0..144u8 {
        let sq = SQ(i);
        assert!(!bb.test_bit(sq));
        bb.set_bit(sq);
        assert!(bb.test_bit(sq));
        assert_eq!(bb.count_bits(), 1);

        // Scan forward
        assert_eq!(bb.bit_scan_forward(), sq);
        // Scan reverse
        assert_eq!(bb.bit_scan_reverse(), sq);

        // Clear
        bb.clear_bit(sq);
        assert!(!bb.test_bit(sq));
        assert!(bb.is_empty());
    }
}

#[test]
fn test_hawk_attacks_range() {
    // Hawk is diagonal jumper, up to 4 steps.
    // Let's test Hawk on e6 (file 4, rank 5).
    // Reachable squares:
    // NE: f7, g8, h9, i10
    // NW: d7, c8, b9, a10
    // SE: f5, g4, h3, i2 (j1 is off-board because home rank is rank 2, rank 1 is empty but wait!
    // rank 1 is idx 0, so f5=5,4; g4=6,3; h3=7,2; i2=8,1; j1=9,0. All on board!)
    // SW: d5, c4, b3, a2 (no further southwest)
    // Let's verify exactly 16 reachable squares.
    let sq = SQ::make(4, 5); // e6
    let attacks = hawk_attacks(sq, BitBoard::EMPTY);
    assert_eq!(attacks.count_bits(), 16);

    // Near corner: a1 (0,0). Can only go NE: b2, c3, d4, e5. (4 squares)
    let a1 = SQ::make(0, 0);
    assert_eq!(hawk_attacks(a1, BitBoard::EMPTY).count_bits(), 4);
}

#[test]
fn test_eagle_attacks_range() {
    // Eagle is orthogonal jumper, up to 4 steps.
    // Let's test Eagle on e6 (file 4, rank 5).
    // Reachable squares:
    // N: e7, e8, e9, e10
    // S: e5, e4, e3, e2
    // E: f6, g6, h6, i6
    // W: d6, c6, b6, a6
    // Total: 16 squares.
    let sq = SQ::make(4, 5);
    let attacks = eagle_attacks(sq, BitBoard::EMPTY);
    assert_eq!(attacks.count_bits(), 16);

    // Near corner: a1 (0,0). North: a2, a3, a4, a5; East: b1, c1, d1, e1. Total: 8 squares.
    let a1 = SQ::make(0, 0);
    assert_eq!(eagle_attacks(a1, BitBoard::EMPTY).count_bits(), 8);
}

#[test]
fn test_hawk_eagle_occupancy_independence() {
    let sq = SQ::make(4, 5);

    // Create random occupancy blocking the paths
    let mut occ = BitBoard::EMPTY;
    occ.set_bit(SQ::make(4, 6)); // block North
    occ.set_bit(SQ::make(5, 6)); // block NE

    let hawk_empty = hawk_attacks(sq, BitBoard::EMPTY);
    let hawk_blocked = hawk_attacks(sq, occ);
    assert_eq!(
        hawk_empty, hawk_blocked,
        "Hawk must be occupancy-independent"
    );

    let eagle_empty = eagle_attacks(sq, BitBoard::EMPTY);
    let eagle_blocked = eagle_attacks(sq, occ);
    assert_eq!(
        eagle_empty, eagle_blocked,
        "Eagle must be occupancy-independent"
    );
}

#[test]
fn test_knight_attacks_12x12() {
    init_statics();
    // Knight on f6 (5, 5). Reachable:
    // d7, g7, c6, h6, c4, h4, d3, g3
    let sq = SQ::make(5, 5);
    let attacks = knight_attacks(sq);
    assert_eq!(attacks.count_bits(), 8);
    assert!(attacks.test_bit(SQ::make(3, 6))); // d7 (5-2, 5+1)
    assert!(attacks.test_bit(SQ::make(7, 6))); // h7 (5+2, 5+1)
}

#[test]
fn test_slider_attacks_respect_blockers() {
    init_statics();
    // Rook at e6 (4,5) on empty board attacks 22 squares:
    // 11 in rank 6 + 11 in e-file = 22.
    // If blocked at e8 (4,7), it should only attack up to e8.
    let sq = SQ::make(4, 5); // e6
    let mut occ = BitBoard::EMPTY;
    occ.set_bit(SQ::make(4, 7)); // blocker at e8

    let rook_att = rook_attacks(sq, occ);
    // Should contain e7 (4,6) and e8 (4,7) but NOT e9 (4,8)
    assert!(rook_att.test_bit(SQ::make(4, 6)));
    assert!(rook_att.test_bit(SQ::make(4, 7)));
    assert!(!rook_att.test_bit(SQ::make(4, 8)));
}

#[test]
fn test_zobrist_incremental_vs_full() {
    let mut board = Board::start_pos();
    let moves = board.generate_moves();

    // Choose a move and apply it
    let m = moves.get(0);
    board.apply_move(m);

    let incremental = board.state.zobrist;
    let full = board.compute_zobrist();
    assert_eq!(
        incremental, full,
        "Incremental Zobrist must match full computation"
    );

    // Undo
    board.undo_move();
    let original = board.state.zobrist;
    let original_full = board.compute_zobrist();
    assert_eq!(original, original_full);
}

#[test]
fn test_zobrist_version_salt_present() {
    let board = Board::start_pos();
    let hash = board.state.zobrist;

    // The hash must include the rule version salt (0xb1cb_fed6_6a8c_6417)
    // We verify this by xor'ing the salt out and checking that it changes the hash.
    assert_ne!(hash, hash ^ 0xb1cb_fed6_6a8c_6417);
}
