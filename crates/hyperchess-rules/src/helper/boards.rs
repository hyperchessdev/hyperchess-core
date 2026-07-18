//! Precomputed attack tables for non-sliding pieces: Knight, King, Pawn attacks.
//! Also Distance, Line, and Between tables.

use crate::core::bitboard::BitBoard;
use crate::core::masks::*;
use crate::core::sq::SQ;

/// Knight attacks from each square.
pub static mut KNIGHT_ATTACKS: [BitBoard; SQ_CNT] = [BitBoard::EMPTY; SQ_CNT];

/// King attacks from each square.
pub static mut KING_ATTACKS: [BitBoard; SQ_CNT] = [BitBoard::EMPTY; SQ_CNT];

/// Pawn attacks: [player][square].
pub static mut PAWN_ATTACKS: [[BitBoard; SQ_CNT]; PLAYER_CNT] =
    [[BitBoard::EMPTY; SQ_CNT]; PLAYER_CNT];

/// Chebyshev distance between squares.
pub static mut DISTANCE: [[u8; SQ_CNT]; SQ_CNT] = [[0u8; SQ_CNT]; SQ_CNT];

/// Line through two squares (full ray in both directions, 0 if not aligned).
pub static mut LINE: [[BitBoard; SQ_CNT]; SQ_CNT] = [[BitBoard::EMPTY; SQ_CNT]; SQ_CNT];

/// Squares strictly between two squares on a line (exclusive).
pub static mut BETWEEN: [[BitBoard; SQ_CNT]; SQ_CNT] = [[BitBoard::EMPTY; SQ_CNT]; SQ_CNT];

/// Initializes all attack/distance tables. Must be called once at startup.
pub fn init_boards() {
    init_knight_attacks();
    init_king_attacks();
    init_pawn_attacks();
    init_distance();
    init_line_between();
}

fn init_knight_attacks() {
    for sq_idx in 0..144u8 {
        let sq = SQ(sq_idx);
        let mut bb = BitBoard::EMPTY;
        let file = sq.file_idx() as i16;
        let rank = sq.rank_idx() as i16;

        for &offset in KNIGHT_OFFSETS.iter() {
            let new_sq = sq_idx as i16 + offset;
            if !(0..144).contains(&new_sq) {
                continue;
            }
            let nf = new_sq % 12;
            let nr = new_sq / 12;
            let fd = (file - nf).abs();
            let rd = (rank - nr).abs();
            // Knight: must move (1,2) or (2,1)
            if (fd == 1 && rd == 2) || (fd == 2 && rd == 1) {
                bb.set_bit(SQ(new_sq as u8));
            }
        }
        unsafe {
            KNIGHT_ATTACKS[sq_idx as usize] = bb;
        }
    }
}

fn init_king_attacks() {
    for sq_idx in 0..144u8 {
        let sq = SQ(sq_idx);
        let mut bb = BitBoard::EMPTY;
        let file = sq.file_idx() as i16;
        let rank = sq.rank_idx() as i16;

        for &offset in KING_OFFSETS.iter() {
            let new_sq = sq_idx as i16 + offset;
            if !(0..144).contains(&new_sq) {
                continue;
            }
            let nf = new_sq % 12;
            let nr = new_sq / 12;
            let fd = (file - nf).abs();
            let rd = (rank - nr).abs();
            // King: max 1 step in each direction
            if fd <= 1 && rd <= 1 {
                bb.set_bit(SQ(new_sq as u8));
            }
        }
        unsafe {
            KING_ATTACKS[sq_idx as usize] = bb;
        }
    }
}

fn init_pawn_attacks() {
    for sq_idx in 0..144u8 {
        let file = (sq_idx % 12) as i16;
        let rank = (sq_idx / 12) as i16;

        // White pawns attack NE, NW
        let mut w_bb = BitBoard::EMPTY;
        for &(df, dr) in &[(1i16, 1i16), (-1, 1)] {
            let nf = file + df;
            let nr = rank + dr;
            if (0..12).contains(&nf) && (0..12).contains(&nr) {
                w_bb.set_bit(SQ((nr * 12 + nf) as u8));
            }
        }
        unsafe {
            PAWN_ATTACKS[0][sq_idx as usize] = w_bb;
        }

        // Black pawns attack SE, SW
        let mut b_bb = BitBoard::EMPTY;
        for &(df, dr) in &[(1i16, -1i16), (-1, -1)] {
            let nf = file + df;
            let nr = rank + dr;
            if (0..12).contains(&nf) && (0..12).contains(&nr) {
                b_bb.set_bit(SQ((nr * 12 + nf) as u8));
            }
        }
        unsafe {
            PAWN_ATTACKS[1][sq_idx as usize] = b_bb;
        }
    }
}

fn init_distance() {
    for sq1 in 0..144usize {
        for sq2 in 0..144usize {
            let f1 = (sq1 % 12) as i16;
            let r1 = (sq1 / 12) as i16;
            let f2 = (sq2 % 12) as i16;
            let r2 = (sq2 / 12) as i16;
            let fd = (f1 - f2).unsigned_abs() as u8;
            let rd = (r1 - r2).unsigned_abs() as u8;
            unsafe {
                DISTANCE[sq1][sq2] = fd.max(rd);
            }
        }
    }
}

fn init_line_between() {
    // For each pair of squares, check if they share a line (rank, file, diagonal, anti-diagonal).
    for sq1 in 0..144usize {
        for sq2 in 0..144usize {
            if sq1 == sq2 {
                continue;
            }
            let f1 = (sq1 % 12) as i16;
            let r1 = (sq1 / 12) as i16;
            let f2 = (sq2 % 12) as i16;
            let r2 = (sq2 / 12) as i16;

            let df = f2 - f1;
            let dr = r2 - r1;

            // Determine if they are aligned (same file, rank, or diagonal)
            let direction: Option<i16> = if df == 0 && dr != 0 {
                // Same file
                Some(if dr > 0 { NORTH } else { SOUTH })
            } else if dr == 0 && df != 0 {
                // Same rank
                Some(if df > 0 { EAST } else { WEST })
            } else if df.abs() == dr.abs() {
                // Diagonal
                if df > 0 && dr > 0 {
                    Some(NORTH_EAST)
                } else if df < 0 && dr > 0 {
                    Some(NORTH_WEST)
                } else if df > 0 && dr < 0 {
                    Some(SOUTH_EAST)
                } else {
                    Some(SOUTH_WEST)
                }
            } else {
                None
            };

            if let Some(dir) = direction {
                // Build LINE: full ray through both squares
                let mut line_bb = BitBoard::EMPTY;
                // Walk from sq1 in direction until off-board
                let mut s = sq1 as i16;
                while (0..144).contains(&s) {
                    line_bb.set_bit(SQ(s as u8));
                    let ns = s + dir;
                    if !(0..144).contains(&ns) {
                        break;
                    }
                    let nf = ns % 12;
                    let of = s % 12;
                    if (nf - of).abs() > 1 {
                        break;
                    }
                    s = ns;
                }
                // Walk from sq1 in opposite direction
                let opp = -dir;
                s = sq1 as i16 + opp;
                while (0..144).contains(&s) {
                    let nf = s % 12;
                    let of = (s - opp) % 12;
                    if (nf - of).abs() > 1 {
                        break;
                    }
                    line_bb.set_bit(SQ(s as u8));
                    s += opp;
                }
                unsafe {
                    LINE[sq1][sq2] = line_bb;
                }

                // Build BETWEEN: squares strictly between sq1 and sq2
                let mut between_bb = BitBoard::EMPTY;
                let step = if df == 0 {
                    if dr > 0 {
                        NORTH
                    } else {
                        SOUTH
                    }
                } else if dr == 0 {
                    if df > 0 {
                        EAST
                    } else {
                        WEST
                    }
                } else {
                    dir
                };

                s = sq1 as i16 + step;
                while s != sq2 as i16 {
                    if !(0..144).contains(&s) {
                        break;
                    }
                    between_bb.set_bit(SQ(s as u8));
                    s += step;
                }
                unsafe {
                    BETWEEN[sq1][sq2] = between_bb;
                }
            }
        }
    }
}

// === Safe accessor functions ===

/// Returns knight attacks from a square.
#[inline(always)]
pub fn knight_attacks(sq: SQ) -> BitBoard {
    unsafe { KNIGHT_ATTACKS[sq.0 as usize] }
}

/// Returns king attacks from a square.
#[inline(always)]
pub fn king_attacks(sq: SQ) -> BitBoard {
    unsafe { KING_ATTACKS[sq.0 as usize] }
}

/// Returns pawn attacks for a player from a square.
#[inline(always)]
pub fn pawn_attacks(player: crate::core::Player, sq: SQ) -> BitBoard {
    unsafe { PAWN_ATTACKS[player as usize][sq.0 as usize] }
}

/// Returns the Chebyshev distance between two squares.
#[inline(always)]
pub fn distance(sq1: SQ, sq2: SQ) -> u8 {
    unsafe { DISTANCE[sq1.0 as usize][sq2.0 as usize] }
}

/// Returns the line through two squares (empty if not aligned).
#[inline(always)]
pub fn line(sq1: SQ, sq2: SQ) -> BitBoard {
    unsafe { LINE[sq1.0 as usize][sq2.0 as usize] }
}

/// Returns the squares between two squares (exclusive, empty if not aligned).
#[inline(always)]
pub fn between(sq1: SQ, sq2: SQ) -> BitBoard {
    unsafe { BETWEEN[sq1.0 as usize][sq2.0 as usize] }
}

/// Returns whether two squares are aligned (on same rank, file, or diagonal).
#[inline(always)]
pub fn aligned(sq1: SQ, sq2: SQ, sq3: SQ) -> bool {
    let l = line(sq1, sq2);
    l.is_not_empty() && l.test_bit(sq3)
}
