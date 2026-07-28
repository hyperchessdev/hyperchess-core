// SPDX-License-Identifier: GPL-3.0-or-later
// HyperChess Core — hyperchess-wasm
// File: crates/hyperchess-wasm/src/lib.rs
// Version: 1.0.0
// Copyright (c) 2026 HyperChess Developer Team

//! HyperChess WASM bindings — two independent wasm-bindgen surfaces sharing
//! one wasm-pack build target:
//! - [`WasmBoard`] (from `board`, formerly `src/hyperchess/src/wasm.rs`) —
//!   rules + search: legal moves, apply-move, all searcher entry points.
//!   Depends on `hyperchess-rules` + `hyperchess-search`.
//! - `Scene3D` (from `scene`/`camera`/`gpu`/`obj`, formerly the standalone
//!   `hyperchess_3d` crate) — WebGPU/WebGL board/piece rendering. Zero
//!   dependency on the rules engine: it takes a 144-byte board encoding as
//!   raw bytes (the same format [`WasmBoard::encode`] produces), not a
//!   `Board` reference — the two surfaces communicate via that shared byte
//!   protocol, not Rust-level coupling. See
//!   docs/hyperchess-core-extraction-plan.md §12 Phase 6 for why these were
//!   merged (source repo kept them as genuinely separate crates/build
//!   pipelines) and for the architectural improvement this crate represents
//!   over the source's "build the rules crate directly with wasm-bindgen
//!   deps" approach (`hyperchess-rules` has no wasm-bindgen dependency at
//!   all — see that crate's Phase 1 notes).
//!
//! `geometry`/`pieces` are plain portable math, also used natively by the
//! `gen_assets` dev-tool binary (`src/bin/gen_assets.rs`) — kept
//! target-agnostic so `cargo build`/`check` (no wasm32 target) still works
//! for that tool. Everything else here only makes sense in a browser.

pub mod geometry;
pub mod pieces;

#[cfg(target_arch = "wasm32")]
mod board;
#[cfg(target_arch = "wasm32")]
mod camera;
#[cfg(target_arch = "wasm32")]
mod gpu;
#[cfg(target_arch = "wasm32")]
mod obj;
#[cfg(target_arch = "wasm32")]
mod scene;

#[cfg(target_arch = "wasm32")]
pub use board::WasmBoard;

#[cfg(target_arch = "wasm32")]
mod wasm_api {
    use crate::scene::Scene;
    use std::cell::RefCell;
    use std::rc::Rc;
    use wasm_bindgen::prelude::*;
    use web_sys::HtmlCanvasElement;

    /// Single combined init for both wasm-bindgen surfaces in this crate —
    /// wasm-bindgen only allows one `#[wasm_bindgen(start)]` function per
    /// crate. Merges what were two separate inits in the source repo
    /// (`hyperchess::wasm`'s panic hook + `Helper::init()`, and
    /// `hyperchess_3d`'s panic hook + `console_log` setup).
    #[wasm_bindgen(start)]
    pub fn init() {
        console_error_panic_hook::set_once();
        let _ = console_log::init_with_level(log::Level::Info);
        hyperchess_rules::Helper::init();
    }

    /// JS-facing handle. Board state is a 144-byte array: 0 = empty, else
    /// `(color << 4) | kind` with kind 1..=8 indexing
    /// `[pawn, knight, bishop, rook, queen, king, eagle, hawk]` and color
    /// 0 = white, 1 = black (see `board3d.js` for the encoder).
    #[wasm_bindgen]
    pub struct Scene3D {
        inner: Rc<RefCell<Scene>>,
    }

    #[wasm_bindgen]
    impl Scene3D {
        /// Creates the renderer against the given canvas. Async because adapter/device
        /// acquisition is a browser Promise under the hood. JS calls this as a static
        /// factory (`await Scene3D.create(canvas)`), not `new Scene3D(...)`.
        pub async fn create(canvas: HtmlCanvasElement) -> Result<Scene3D, JsValue> {
            let scene = Scene::new(canvas)
                .await
                .map_err(|e| JsValue::from_str(&e))?;
            Ok(Scene3D {
                inner: Rc::new(RefCell::new(scene)),
            })
        }

        /// Draw one frame. JS drives the render loop, so this must be called
        /// from `requestAnimationFrame` — the renderer never schedules itself.
        pub fn render(&self) {
            self.inner.borrow_mut().render();
        }

        /// Match the swapchain to a new canvas size, in physical pixels.
        pub fn resize(&self, width: u32, height: u32) {
            self.inner.borrow_mut().resize(width, height);
        }

        /// Rotate the camera around the board by a *delta* in radians;
        /// pitch is clamped internally so the camera cannot flip over the pole.
        pub fn orbit(&self, dyaw: f32, dpitch: f32) {
            self.inner.borrow_mut().orbit(dyaw, dpitch);
        }

        /// Scale the camera distance by `factor` (`>1` pulls back, `<1` moves
        /// in), clamped to the scene's usable range.
        pub fn zoom(&self, factor: f32) {
            self.inner.borrow_mut().zoom(factor);
        }

        /// `board` must be exactly 144 bytes (see the encoding note on `Scene3D`).
        pub fn set_board(&self, board: &[u8], flipped: bool) {
            self.inner.borrow_mut().set_board(board, flipped);
        }

        /// Starts a glide animation for the piece on `from` moving to `to`. Call
        /// this *before* `set_board` with the post-move position — it snapshots
        /// the pre-move board to know what to animate (and what's being captured).
        pub fn animate_move(&self, from: u8, to: u8) {
            self.inner.borrow_mut().animate_move(from, to);
        }

        /// True while a move animation is still in flight — keep calling
        /// `render()` on every frame until this goes false, then it's fine to
        /// idle the render loop until the next interaction.
        pub fn is_animating(&self) -> bool {
            self.inner.borrow().is_animating()
        }

        /// `-1` means "none" for `selected`/`last_from`/`last_to`/`checked_king`.
        pub fn set_selection(
            &self,
            selected: i32,
            legal: &[u8],
            last_from: i32,
            last_to: i32,
            checked_king: i32,
        ) {
            let opt_sq = |v: i32| {
                if (0..144).contains(&v) {
                    Some(v as u8)
                } else {
                    None
                }
            };
            let last_move = match (opt_sq(last_from), opt_sq(last_to)) {
                (Some(f), Some(t)) => Some((f, t)),
                _ => None,
            };
            self.inner.borrow_mut().set_selection(
                opt_sq(selected),
                legal.to_vec(),
                last_move,
                opt_sq(checked_king),
            );
        }

        /// Canvas-space pixel coordinates -> square index (0-143), or -1 if off-board.
        pub fn pick(&self, x: f32, y: f32) -> i32 {
            self.inner
                .borrow()
                .pick(x, y)
                .map(|sq| sq as i32)
                .unwrap_or(-1)
        }
    }
}

#[cfg(target_arch = "wasm32")]
pub use wasm_api::Scene3D;
