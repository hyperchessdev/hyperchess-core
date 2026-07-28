// SPDX-License-Identifier: GPL-3.0-or-later
// HyperChess Core — hyperchess-wasm
// File: crates/hyperchess-wasm/src/pieces.rs
// Version: 1.0.0
// Copyright (c) 2026 HyperChess Developer Team

//! Per-piece-type placeholder mesh composition. Each function builds one piece,
//! centered at (0,0,0) on the XZ plane with its base sitting at y=0, out of the
//! primitives in `geometry.rs`. Purely geometric — color/material comes from the
//! renderer's per-instance tint, not from these meshes.
//!
//! HyperChess's two invented pieces (Eagle, Hawk — short-range leapers, see
//! docs/Hyperchess_Rules_v3.md) have no classical silhouette to follow, so their
//! shapes here are original: Eagle = twin spread wings, Hawk = single dorsal fin.

use crate::geometry::{cone, cuboid, cylinder, frustum, torus, uv_sphere, Mesh};
use glam::{Mat4, Quat, Vec3};

/// Radial segment count for every revolved primitive. Low enough to keep the
/// faceted look consistent across pieces, high enough that a base does not read
/// as a polygon.
const SEG: u32 = 14;
/// Shared plinth radius — uniform across piece types so pieces line up on a
/// square regardless of body width.
const BASE_R: f32 = 0.30;
/// Shared plinth height. Every body starts at this `y`, which is why the piece
/// builders below all offset from `BASE_H` rather than 0.
const BASE_H: f32 = 0.09;

/// Shorthand for a translate/rotate/scale matrix, applied in SRT order.
fn xf(t: Vec3, r: Quat, s: Vec3) -> Mat4 {
    Mat4::from_scale_rotation_translation(s, r, t)
}

/// The common plinth every piece sits on.
fn base() -> Mesh {
    cylinder(Vec3::ZERO, BASE_R, BASE_H, SEG)
}

/// Pawn: tapered body with a spherical head.
pub fn pawn() -> Mesh {
    let body = frustum(Vec3::new(0.0, BASE_H, 0.0), 0.20, 0.13, 0.26, SEG);
    let head = uv_sphere(Vec3::new(0.0, BASE_H + 0.26 + 0.11, 0.0), 0.14, 8, SEG);
    Mesh::merge([base(), body, head])
}

/// Rook: straight shaft under a battlement of four corner merlons.
pub fn rook() -> Mesh {
    let shaft = cylinder(Vec3::new(0.0, BASE_H, 0.0), 0.23, 0.38, SEG);
    let collar = frustum(Vec3::new(0.0, BASE_H + 0.38, 0.0), 0.23, 0.29, 0.05, SEG);
    let top = cylinder(Vec3::new(0.0, BASE_H + 0.43, 0.0), 0.29, 0.09, SEG);
    let top_y = BASE_H + 0.43 + 0.09;
    let merlons: Vec<Mesh> = (0..4)
        .map(|i| {
            let a = i as f32 / 4.0 * std::f32::consts::TAU + std::f32::consts::FRAC_PI_4;
            let pos = Vec3::new(a.cos() * 0.20, top_y + 0.05, a.sin() * 0.20);
            cuboid(pos, Vec3::splat(0.11))
        })
        .collect();
    Mesh::merge(std::iter::once(Mesh::merge([shaft, collar, top])).chain(merlons))
}

/// Knight: angled head and muzzle on a tapered neck, with a single swept ear.
pub fn knight() -> Mesh {
    let neck = frustum(Vec3::new(0.0, BASE_H, 0.0), 0.19, 0.15, 0.32, SEG);
    let neck_top = BASE_H + 0.32;
    let head = cuboid(Vec3::ZERO, Vec3::new(0.16, 0.30, 0.22)).transformed(xf(
        Vec3::new(0.0, neck_top + 0.12, 0.06),
        Quat::from_rotation_x(-0.55),
        Vec3::ONE,
    ));
    let muzzle = cuboid(Vec3::ZERO, Vec3::new(0.11, 0.13, 0.20)).transformed(xf(
        Vec3::new(0.0, neck_top + 0.10, 0.20),
        Quat::from_rotation_x(-0.15),
        Vec3::ONE,
    ));
    let ear = cone(Vec3::ZERO, 0.045, 0.16, 8).transformed(xf(
        Vec3::new(0.0, neck_top + 0.30, -0.02),
        Quat::from_rotation_x(-0.4),
        Vec3::ONE,
    ));
    Mesh::merge([base(), neck, head, muzzle, ear])
}

/// Bishop: tall narrow taper capped by a sphere and a finial.
pub fn bishop() -> Mesh {
    let body = frustum(Vec3::new(0.0, BASE_H, 0.0), 0.21, 0.09, 0.46, SEG);
    let cap = uv_sphere(Vec3::new(0.0, BASE_H + 0.46 + 0.08, 0.0), 0.10, 8, SEG);
    let finial = cone(Vec3::new(0.0, BASE_H + 0.46 + 0.16, 0.0), 0.03, 0.10, 8);
    Mesh::merge([base(), body, cap, finial])
}

/// `count` spheres evenly spaced on a horizontal circle — the crown points
/// shared by the queen's coronet.
fn crown_ring(y: f32, count: u32, radius: f32, sphere_r: f32) -> Vec<Mesh> {
    (0..count)
        .map(|i| {
            let a = i as f32 / count as f32 * std::f32::consts::TAU;
            uv_sphere(
                Vec3::new(a.cos() * radius, y, a.sin() * radius),
                sphere_r,
                6,
                8,
            )
        })
        .collect()
}

/// Queen: wide taper, collar torus, six-point crown, and a central orb.
pub fn queen() -> Mesh {
    let body = frustum(Vec3::new(0.0, BASE_H, 0.0), 0.25, 0.135, 0.54, SEG);
    let top_y = BASE_H + 0.54;
    let collar = torus(Vec3::new(0.0, top_y, 0.0), 0.15, 0.035, SEG, 8);
    let points = crown_ring(top_y + 0.05, 6, 0.15, 0.055);
    let orb = uv_sphere(Vec3::new(0.0, top_y + 0.16, 0.0), 0.08, 8, SEG);
    Mesh::merge(std::iter::once(Mesh::merge([base(), body, collar, orb])).chain(points))
}

/// King: like the queen but taller, topped with an orb and cross instead of a
/// crown ring.
pub fn king() -> Mesh {
    let body = frustum(Vec3::new(0.0, BASE_H, 0.0), 0.25, 0.145, 0.60, SEG);
    let top_y = BASE_H + 0.60;
    let collar = torus(Vec3::new(0.0, top_y, 0.0), 0.16, 0.035, SEG, 8);
    let orb = uv_sphere(Vec3::new(0.0, top_y + 0.09, 0.0), 0.09, 8, SEG);
    let cross_v = cuboid(
        Vec3::new(0.0, top_y + 0.24, 0.0),
        Vec3::new(0.06, 0.22, 0.06),
    );
    let cross_h = cuboid(
        Vec3::new(0.0, top_y + 0.24, 0.0),
        Vec3::new(0.18, 0.06, 0.06),
    );
    Mesh::merge([base(), body, collar, orb, cross_v, cross_h])
}

/// Eagle (HyperChess-original short-range leaper): body + a pair of spread wings.
pub fn eagle() -> Mesh {
    let body = cone(Vec3::new(0.0, BASE_H, 0.0), 0.20, 0.42, SEG);
    let body_top = BASE_H + 0.42;
    let head = uv_sphere(Vec3::new(0.0, body_top - 0.02, 0.10), 0.10, 8, SEG);
    let beak = cone(Vec3::ZERO, 0.04, 0.13, 8).transformed(xf(
        Vec3::new(0.0, body_top - 0.02, 0.21),
        Quat::from_rotation_x(std::f32::consts::FRAC_PI_2),
        Vec3::ONE,
    ));
    let wing_l = cuboid(Vec3::ZERO, Vec3::new(0.42, 0.045, 0.20)).transformed(xf(
        Vec3::new(-0.24, BASE_H + 0.24, -0.02),
        Quat::from_rotation_z(0.55) * Quat::from_rotation_x(-0.2),
        Vec3::ONE,
    ));
    let wing_r = cuboid(Vec3::ZERO, Vec3::new(0.42, 0.045, 0.20)).transformed(xf(
        Vec3::new(0.24, BASE_H + 0.24, -0.02),
        Quat::from_rotation_z(-0.55) * Quat::from_rotation_x(-0.2),
        Vec3::ONE,
    ));
    Mesh::merge([base(), body, head, beak, wing_l, wing_r])
}

/// Hawk (HyperChess-original short-range leaper, sibling of Eagle): sleeker body
/// with a single dorsal fin instead of spread wings, so the two read as distinct
/// pieces at a glance rather than recolors of the same shape.
pub fn hawk() -> Mesh {
    let body = frustum(Vec3::new(0.0, BASE_H, 0.0), 0.18, 0.07, 0.36, SEG);
    let body_top = BASE_H + 0.36;
    let head = uv_sphere(Vec3::new(0.0, body_top, 0.09), 0.09, 8, SEG);
    let beak = cone(Vec3::ZERO, 0.035, 0.11, 8).transformed(xf(
        Vec3::new(0.0, body_top, 0.19),
        Quat::from_rotation_x(std::f32::consts::FRAC_PI_2),
        Vec3::ONE,
    ));
    let fin = cuboid(Vec3::ZERO, Vec3::new(0.045, 0.34, 0.24)).transformed(xf(
        Vec3::new(0.0, BASE_H + 0.30, -0.08),
        Quat::from_rotation_x(-0.35),
        Vec3::ONE,
    ));
    Mesh::merge([base(), body, head, beak, fin])
}

/// Build the mesh for a piece by lowercase name, or `None` for an unknown one.
///
/// Accepts exactly the strings in [`PIECE_NAMES`]; the asset generator iterates
/// that array and feeds each entry here, so the two must stay in agreement.
pub fn mesh_for(name: &str) -> Option<Mesh> {
    Some(match name {
        "pawn" => pawn(),
        "rook" => rook(),
        "knight" => knight(),
        "bishop" => bishop(),
        "queen" => queen(),
        "king" => king(),
        "eagle" => eagle(),
        "hawk" => hawk(),
        _ => return None,
    })
}

/// Every piece type this module can build, in the crate's canonical order.
///
/// The order is load-bearing, not cosmetic: it is the `kind` numbering of the
/// 144-byte board encoding documented on `Scene3D`, offset by one (kind 1 is
/// index 0). Reordering this array silently remaps every piece the renderer
/// draws.
pub const PIECE_NAMES: [&str; 8] = [
    "pawn", "knight", "bishop", "rook", "queen", "king", "eagle", "hawk",
];
