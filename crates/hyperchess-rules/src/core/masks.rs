//! Masks and constants for the 12x12 HyperChess board.

/// Total number of players.
pub const PLAYER_CNT: usize = 2;
/// Total number of piece types (None, P, N, B, R, Q, K, E, H, All).
pub const PIECE_TYPE_CNT: usize = 10;
/// Total number of piece+player combinations.
pub const PIECE_CNT: usize = 20;
/// Total number of squares on the 12x12 board.
pub const SQ_CNT: usize = 144;
/// Total number of files (a-l).
pub const FILE_CNT: usize = 12;
/// Total number of ranks (1-12).
pub const RANK_CNT: usize = 12;
/// Maximum number of moves in a position.
pub const MAX_MOVES: usize = 512;

/// Game phases.
pub const PHASE_CNT: usize = 2;
/// Castling sides (king-side, queen-side).
pub const CASTLING_SIDES: usize = 2;
/// Total castling combinations per player.
pub const TOTAL_CASTLING_CNT: usize = 4;
/// Total castling rights for both players.
pub const ALL_CASTLING_RIGHTS: usize = 16;

// === Directions (12-wide board) ===

/// Direction north on a 12-wide board.
pub const NORTH: i16 = 12;
/// Direction south on a 12-wide board.
pub const SOUTH: i16 = -12;
/// Direction east.
pub const EAST: i16 = 1;
/// Direction west.
pub const WEST: i16 = -1;
/// Direction northeast.
pub const NORTH_EAST: i16 = 13;
/// Direction northwest.
pub const NORTH_WEST: i16 = 11;
/// Direction southeast.
pub const SOUTH_EAST: i16 = -11;
/// Direction southwest.
pub const SOUTH_WEST: i16 = -13;

// === Castling squares (12x12) ===
// King starts on g-file (file 6), QRook on c-file (file 2), KRook on j-file (file 9)
// rank 2 for White (rank_idx=1), rank 11 for Black (rank_idx=10)

/// White king starting square: g2 = 1*12 + 6 = 18
pub const WHITE_KING_START: u8 = 18;
/// Black king starting square: g11 = 10*12 + 6 = 126
pub const BLACK_KING_START: u8 = 126;

/// White king-side rook: j2 = 1*12 + 9 = 21
pub const ROOK_WHITE_KSIDE_START: u8 = 21;
/// White queen-side rook: c2 = 1*12 + 2 = 14
pub const ROOK_WHITE_QSIDE_START: u8 = 14;
/// Black king-side rook: j11 = 10*12 + 9 = 129
pub const ROOK_BLACK_KSIDE_START: u8 = 129;
/// Black queen-side rook: c11 = 10*12 + 2 = 122
pub const ROOK_BLACK_QSIDE_START: u8 = 122;

/// Castling right bits.
pub const C_WHITE_K_MASK: u8 = 0b0000_1000;
pub const C_WHITE_Q_MASK: u8 = 0b0000_0100;
pub const C_BLACK_K_MASK: u8 = 0b0000_0010;
pub const C_BLACK_Q_MASK: u8 = 0b0000_0001;

/// Starting rook positions: [player][side] where side 0=king, 1=queen.
pub static CASTLING_ROOK_START: [[u8; CASTLING_SIDES]; PLAYER_CNT] = [
    [ROOK_WHITE_KSIDE_START, ROOK_WHITE_QSIDE_START],
    [ROOK_BLACK_KSIDE_START, ROOK_BLACK_QSIDE_START],
];

/// King starting positions per player.
pub static KING_START: [u8; PLAYER_CNT] = [WHITE_KING_START, BLACK_KING_START];

// King-side castle: King g2→i2, Rook j2→h2
// Queen-side castle: King g2→e2, Rook c2→f2
// King destination squares for castling [player][side]
pub static CASTLING_KING_DST: [[u8; CASTLING_SIDES]; PLAYER_CNT] = [
    [20, 16],   // White: i2=20, e2=16
    [128, 124], // Black: i11=128, e11=124
];

// Rook destination squares for castling [player][side]
pub static CASTLING_ROOK_DST: [[u8; CASTLING_SIDES]; PLAYER_CNT] = [
    [19, 17],   // White: h2=19, f2=17
    [127, 125], // Black: h11=127, f11=125
];

// === File characters ===
pub static FILE_DISPLAYS: [char; FILE_CNT] =
    ['a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l'];

// === Rank characters ===
pub static RANK_DISPLAYS: [char; RANK_CNT] =
    ['1', '2', '3', '4', '5', '6', '7', '8', '9', 'T', 'E', 'W'];

/// Piece display characters [player][piece_type].
pub static PIECE_DISPLAYS: [[char; PIECE_TYPE_CNT]; PLAYER_CNT] = [
    ['_', 'P', 'N', 'B', 'R', 'Q', 'K', 'E', 'H', '*'],
    ['_', 'p', 'n', 'b', 'r', 'q', 'k', 'e', 'h', '*'],
];

/// HFEN piece characters for parsing.
pub static HFEN_PIECE_CHARS: &str = "PNBRQKEHpnbrqkeh";

/// Display order for squares (top-to-bottom, left-to-right for display).
pub fn sq_display_order() -> Vec<u8> {
    let mut order = Vec::with_capacity(SQ_CNT);
    for rank in (0..12u8).rev() {
        for file in 0..12u8 {
            order.push(rank * 12 + file);
        }
    }
    order
}

/// Starting HFEN for HyperChess.
pub const START_HFEN: &str = super::piece_identity::START_HFEN_I;

/// Knight move offsets for a 12-wide board.
pub static KNIGHT_OFFSETS: [i16; 8] = [
    2 * NORTH + EAST, // +25
    2 * NORTH + WEST, // +23
    NORTH + 2 * EAST, // +14
    NORTH + 2 * WEST, // +10
    2 * SOUTH + EAST, // -23
    2 * SOUTH + WEST, // -25
    SOUTH + 2 * EAST, // -10
    SOUTH + 2 * WEST, // -14
];

/// King move offsets for a 12-wide board.
pub static KING_OFFSETS: [i16; 8] = [
    NORTH, SOUTH, EAST, WEST, NORTH_EAST, NORTH_WEST, SOUTH_EAST, SOUTH_WEST,
];

/// Orthogonal ray directions (for Rook/Eagle).
pub static ROOK_DIRS: [i16; 4] = [NORTH, SOUTH, EAST, WEST];

/// Diagonal ray directions (for Bishop/Hawk).
pub static BISHOP_DIRS: [i16; 4] = [NORTH_EAST, NORTH_WEST, SOUTH_EAST, SOUTH_WEST];

/// All 8 ray directions.
pub static ALL_DIRS: [i16; 8] = [
    NORTH, SOUTH, EAST, WEST, NORTH_EAST, NORTH_WEST, SOUTH_EAST, SOUTH_WEST,
];
