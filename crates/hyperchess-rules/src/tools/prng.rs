// SPDX-License-Identifier: GPL-3.0-or-later
// HyperChess Core — hyperchess-rules
// File: crates/hyperchess-rules/src/tools/prng.rs
// Version: 1.0.0
// Copyright (c) 2026 HyperChess Developer Team

//! Pseudo-random number generator (reused from PlecoDB).

/// PRNG based on XorShift64*.
pub struct PRNG {
    state: u64,
}

impl PRNG {
    /// Creates a new PRNG with the given seed.
    #[inline]
    pub fn init(seed: u64) -> Self {
        debug_assert!(seed != 0);
        PRNG { state: seed }
    }

    /// Returns the next pseudo-random u64.
    #[inline]
    pub fn rand(&mut self) -> u64 {
        self.state ^= self.state >> 12;
        self.state ^= self.state << 25;
        self.state ^= self.state >> 27;
        self.state.wrapping_mul(2685821657736338717)
    }

    /// Returns a sparse random u64 (few bits set).
    #[inline]
    pub fn sparse_rand(&mut self) -> u64 {
        self.rand() & self.rand() & self.rand()
    }

    /// Returns a u64 with exactly one bit set.
    #[inline]
    pub fn singular_bit(&mut self) -> u64 {
        1u64 << (self.rand() & 63)
    }
}
