// SPDX-License-Identifier: GPL-3.0-or-later
// HyperChess Core — hyperchess-rules
// File: crates/hyperchess-rules/src/helper/mod.rs
// Version: 1.0.0
// Copyright (c) 2026 HyperChess Developer Team

//! Helper module — lookup tables and attack generation.

pub mod boards;
pub mod jumping;
pub mod prelude;
pub mod psqt;
pub mod sliding;
pub mod zobrist;

/// Helper struct for initialization.
pub struct Helper;

impl Helper {
    /// Initialize all static tables. Must be called before using the engine.
    pub fn init() {
        prelude::init_statics();
    }
}
