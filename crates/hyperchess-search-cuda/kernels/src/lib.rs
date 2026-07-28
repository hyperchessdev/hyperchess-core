// SPDX-License-Identifier: GPL-3.0-or-later
// HyperChess Core — hyperchess_kernels
// File: crates/hyperchess-search-cuda/kernels/src/lib.rs
// Version: 1.0.0
// Copyright (c) 2026 HyperChess Developer Team

#![cfg_attr(target_os = "cuda", no_std)]
#![allow(clippy::missing_safety_doc)]

pub mod eval;
