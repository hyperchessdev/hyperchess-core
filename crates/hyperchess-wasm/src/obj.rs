//! Minimal Wavefront OBJ parser: just enough to load the placeholder piece meshes
//! (and, later, real artist-modeled replacements dropped into `src/assets/pieces/`
//! with the same filenames). Supports `v`, `vn`, and triangulated/fan-triangulated
//! `f` faces with `v`, `v//vn`, `v/vt/vn`, or `v/vt` indices. Faces missing normals
//! get a flat per-face normal computed on the fly, so plain "positions + faces"
//! exports (common from quick modeling-tool exports) still work.

use crate::geometry::Mesh;
use glam::Vec3;

pub fn parse(src: &str) -> Mesh {
    let mut positions: Vec<Vec3> = Vec::new();
    let mut normals: Vec<Vec3> = Vec::new();
    let mut mesh = Mesh::default();

    for line in src.lines() {
        let line = line.trim();
        let mut it = line.split_whitespace();
        match it.next() {
            Some("v") => {
                let v: Vec<f32> = it.filter_map(|t| t.parse().ok()).collect();
                if v.len() >= 3 {
                    positions.push(Vec3::new(v[0], v[1], v[2]));
                }
            }
            Some("vn") => {
                let v: Vec<f32> = it.filter_map(|t| t.parse().ok()).collect();
                if v.len() >= 3 {
                    normals.push(Vec3::new(v[0], v[1], v[2]));
                }
            }
            Some("f") => {
                let verts: Vec<(usize, Option<usize>)> = it.filter_map(parse_face_token).collect();
                // Fan-triangulate polygons with more than 3 vertices.
                for i in 1..verts.len().saturating_sub(1) {
                    let tri = [verts[0], verts[i], verts[i + 1]];
                    let p: Vec<Vec3> = tri.iter().map(|(pi, _)| positions[*pi]).collect();
                    let face_normal = (p[1] - p[0]).cross(p[2] - p[0]).normalize_or_zero();
                    let base = mesh.positions.len() as u32;
                    for (k, (_pi, ni)) in tri.iter().enumerate() {
                        let n = ni
                            .and_then(|i| normals.get(i))
                            .copied()
                            .unwrap_or(face_normal);
                        mesh.positions.push(p[k].to_array());
                        mesh.normals.push(n.to_array());
                    }
                    mesh.indices.extend([base, base + 1, base + 2]);
                }
            }
            _ => {}
        }
    }
    mesh
}

/// Parses one `f` face token (`v`, `v/vt`, `v/vt/vn`, or `v//vn`) into
/// 0-based (position_index, normal_index) — OBJ indices are 1-based, and
/// negative indices count back from the end of the list seen so far.
fn parse_face_token(tok: &str) -> Option<(usize, Option<usize>)> {
    let mut parts = tok.split('/');
    let pi = parts.next()?.parse::<i64>().ok()?;
    let vi = to_index(pi);
    let ni = parts
        .nth(1)
        .and_then(|s| s.parse::<i64>().ok())
        .map(to_index);
    Some((vi, ni))
}

fn to_index(raw: i64) -> usize {
    // OBJ is 1-based; we don't support negative (relative) indices here since
    // our generator and typical exporters both emit absolute positive indices.
    (raw - 1).max(0) as usize
}
