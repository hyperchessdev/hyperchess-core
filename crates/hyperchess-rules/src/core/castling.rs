//! Castling side and game-phase enums.

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum CastleType {
    KingSide = 0,
    QueenSide = 1,
}

/// Game phases.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum Phase {
    MG = 0,
    EG = 1,
}
