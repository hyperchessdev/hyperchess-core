// SPDX-License-Identifier: GPL-3.0-or-later
// HyperChess Core — hyperchess-driver
// File: crates/hyperchess-driver/src/uci/pool.rs
// Version: 1.0.0
// Copyright (c) 2026 HyperChess Developer Team

//! Engine pool for parallel data generation.
//!
//! Wraps `N` UCI engine instances behind `Arc<Mutex<UciEngine>>` handles.
//! Acquisition uses a round-robin index with an `AtomicUsize` so multiple
//! async tasks get different engines without blocking.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::Mutex;

use anyhow::Result;

use super::client::UciEngine;

/// A pool of UCI engine subprocesses for parallel analysis.
pub struct EnginePool {
    engines: Vec<Arc<Mutex<UciEngine>>>,
    cursor: AtomicUsize,
}

impl EnginePool {
    /// Create a pool of `size` identical UCI engine instances.
    ///
    /// `extra_opts` are forwarded to each `UciEngine::new()` call.
    pub async fn new(bin: &str, extra_opts: &[(&str, &str)], size: usize) -> Result<Self> {
        let mut engines = Vec::with_capacity(size);
        for _ in 0..size {
            let engine = UciEngine::new(bin, extra_opts).await?;
            engines.push(Arc::new(Mutex::new(engine)));
        }
        Ok(Self {
            engines,
            cursor: AtomicUsize::new(0),
        })
    }

    /// Create a pool using the native HyperChess UCI server.
    pub async fn native(uci_bin: &str, size: usize) -> Result<Self> {
        Self::new(uci_bin, &[], size).await
    }

    /// Acquire an engine handle (round-robin, non-blocking).
    pub fn acquire(&self) -> Arc<Mutex<UciEngine>> {
        let idx = self.cursor.fetch_add(1, Ordering::Relaxed) % self.engines.len();
        self.engines[idx].clone()
    }

    /// Number of engines in the pool.
    pub fn size(&self) -> usize {
        self.engines.len()
    }
}
