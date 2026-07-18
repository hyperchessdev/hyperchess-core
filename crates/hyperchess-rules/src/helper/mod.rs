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
