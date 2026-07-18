//! UCI (Universal Chess Interface) integration.
//!
//! - [`server`] + [`server_util`] — the native HyperChess UCI server (the
//!   `hyperchess uci` subcommand): reads UCI commands from stdin, runs the
//!   native Rust engine, writes UCI responses to stdout.
//! - [`client`] — async UCI client for spawning and talking to any
//!   UCI-speaking engine subprocess (not just the native server above).
//! - [`pool`] — a pool of [`client::UciEngine`] instances for parallel
//!   analysis.
//! - [`calibration`] — piece-value calibration via self-play (developer tool,
//!   not exposed as a CLI subcommand).

pub mod calibration;
pub mod client;
pub mod pool;
pub mod server;
pub mod server_util;

pub use server::run;
