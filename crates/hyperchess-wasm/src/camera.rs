// SPDX-License-Identifier: GPL-3.0-or-later
// HyperChess Core — hyperchess-wasm
// File: crates/hyperchess-wasm/src/camera.rs
// Version: 1.0.0
// Copyright (c) 2026 HyperChess Developer Team

//! Orbit camera around the board center. Input handling (drag-to-orbit vs.
//! click-to-select, wheel-to-zoom) lives in JS (board3d.js); this just holds the
//! spherical parameters and does the view/projection math + screen picking.

use glam::{Mat4, Vec3};

/// Board width in world units — one unit per square on the 12x12 grid.
pub const BOARD_SPAN: f32 = 12.0;
/// World-space point the camera orbits: the board's midpoint at ground level.
pub const BOARD_CENTER: Vec3 = Vec3::new(BOARD_SPAN * 0.5, 0.0, BOARD_SPAN * 0.5);

/// Lowest pitch, keeping the camera above the board plane — at 0 the view
/// would be edge-on and the board would collapse to a line.
const MIN_PITCH: f32 = 0.20;
/// Highest pitch, just short of straight down, so the scene keeps some
/// perspective and pieces stay distinguishable from their bases.
const MAX_PITCH: f32 = 1.45;
/// Closest zoom, before the near plane starts clipping pieces.
const MIN_DIST: f32 = 6.0;
/// Furthest zoom, past which the board no longer fills a useful part of the
/// viewport.
const MAX_DIST: f32 = 28.0;

/// Orbit camera in spherical coordinates about [`Camera::target`].
///
/// Stored as yaw/pitch/distance rather than as a matrix so the clamping that
/// keeps the view usable is expressed directly on the parameters.
pub struct Camera {
    /// Point the camera looks at and orbits around.
    pub target: Vec3,
    /// Rotation about the vertical axis, in radians.
    pub yaw: f32,
    /// Elevation above the board plane, in radians, clamped to
    /// `MIN_PITCH..=MAX_PITCH`.
    pub pitch: f32,
    /// Distance from `target`, clamped to `MIN_DIST..=MAX_DIST`.
    pub distance: f32,
    /// Vertical field of view, in radians.
    pub fov_y: f32,
}

impl Camera {
    /// The camera always sits on the same side (`yaw = 0`); a flipped board is
    /// achieved by mirroring square placement (see `scene::square_world_xz`), not
    /// by moving the camera — so "front row, near the camera" is always whichever
    /// side is playing from this device, in both orientations.
    pub fn default() -> Self {
        Self {
            target: BOARD_CENTER,
            yaw: 0.0,
            pitch: 0.95,
            distance: 15.0,
            fov_y: 45f32.to_radians(),
        }
    }

    /// World-space camera position, derived from the spherical parameters.
    pub fn eye(&self) -> Vec3 {
        let (sy, cy) = self.yaw.sin_cos();
        let (sp, cp) = self.pitch.sin_cos();
        self.target + self.distance * Vec3::new(sy * cp, sp, cy * cp)
    }

    /// Apply a rotation delta in radians. Yaw wraps freely; pitch is clamped so
    /// the camera can never pass over the pole or below the board.
    pub fn orbit(&mut self, dyaw: f32, dpitch: f32) {
        self.yaw += dyaw;
        self.pitch = (self.pitch + dpitch).clamp(MIN_PITCH, MAX_PITCH);
    }

    /// Scale the orbit distance by `factor` (`>1` pulls back), clamped to the
    /// usable range. Multiplicative so each wheel notch feels the same at any
    /// distance.
    pub fn zoom(&mut self, factor: f32) {
        self.distance = (self.distance * factor).clamp(MIN_DIST, MAX_DIST);
    }

    /// Right-handed view matrix looking from [`Camera::eye`] at the target.
    pub fn view_matrix(&self) -> Mat4 {
        Mat4::look_at_rh(self.eye(), self.target, Vec3::Y)
    }

    /// Perspective projection. `aspect` is floored at 0.01 so a zero-width
    /// canvas yields a degenerate but finite matrix rather than NaNs.
    pub fn proj_matrix(&self, aspect: f32) -> Mat4 {
        Mat4::perspective_rh(self.fov_y, aspect.max(0.01), 0.1, 100.0)
    }

    /// Combined projection * view, as uploaded to the camera uniform buffer.
    pub fn view_proj(&self, aspect: f32) -> Mat4 {
        self.proj_matrix(aspect) * self.view_matrix()
    }

    /// Casts a ray from the camera through the given normalized-device-coordinates
    /// point (`ndc_x`, `ndc_y` each in -1..1, +Y up) and intersects it with the
    /// board's y=0 plane. Returns the world-space hit point, if the ray isn't
    /// parallel to the plane.
    pub fn plane_hit(&self, ndc_x: f32, ndc_y: f32, aspect: f32) -> Option<Vec3> {
        let inv = (self.proj_matrix(aspect) * self.view_matrix()).inverse();
        let near = inv.project_point3(Vec3::new(ndc_x, ndc_y, 0.0));
        let far = inv.project_point3(Vec3::new(ndc_x, ndc_y, 1.0));
        let dir = far - near;
        if dir.y.abs() < 1e-6 {
            return None;
        }
        let t = -near.y / dir.y;
        if t < 0.0 {
            return None;
        }
        Some(near + dir * t)
    }
}
