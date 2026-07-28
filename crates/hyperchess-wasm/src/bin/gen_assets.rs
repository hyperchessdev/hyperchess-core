//! Generates the placeholder primitive piece models under `assets/pieces/*.obj`.
//!
//! Run with `cargo run -p hyperchess-wasm --bin gen_assets`. Output files are plain
//! OBJ text (positions + normals, no materials — color comes from the renderer's
//! per-instance white/black tint) so replacing a file with a real modeled OBJ of
//! the same name is a drop-in swap; no code changes needed.

use hyperchess_wasm::pieces;
use std::path::PathBuf;

fn main() {
    // NOTE vs. the source repo: `assets/` moved from a sibling of the crate
    // (src/hyperchess_3d/../assets/) to a child of it (this crate's own
    // assets/) during the Phase 6 extraction — "../assets/pieces" would
    // silently write outside this crate (caught by actually running this
    // binary, not by inspection: it wrote a stray crates/assets/pieces/
    // directory one level up from where it should have).
    let out_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/pieces");
    std::fs::create_dir_all(&out_dir).expect("create assets/pieces");

    for name in pieces::PIECE_NAMES {
        let mesh = pieces::mesh_for(name).expect("known piece name");
        let path = out_dir.join(format!("{name}.obj"));
        std::fs::write(&path, mesh.to_obj()).unwrap_or_else(|e| panic!("write {path:?}: {e}"));
        println!(
            "wrote {path:?} ({} verts, {} tris)",
            mesh.positions.len(),
            mesh.indices.len() / 3
        );
    }
}
