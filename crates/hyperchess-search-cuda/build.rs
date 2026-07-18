use std::{env, path};

fn main() {
    println!("cargo::rerun-if-changed=build.rs");
    println!("cargo::rerun-if-changed=kernels");

    let out_path = path::PathBuf::from(env::var("OUT_DIR").unwrap());
    let ptx_out = out_path.join("hyperchess_kernels.ptx");

    // Only compile PTX when the `cuda` feature is active (which also enables cuda_builder).
    #[cfg(feature = "cuda")]
    {
        let manifest_dir = path::PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());

        // The llvm19 feature compiles via LLVM 19 PTX backend targeting sm_100
        // (Blackwell) by default. The A6000 is Ampere (sm_86); its driver JIT
        // cannot compile PTX ISA 9.1 targeting sm_100 (error 222).
        // sm_80/sm_86 with llvm19 fall back to NVVM which rejects LLVM 19
        // bitcode (LLVM 7.0.1 reader mismatch — ICE).
        //
        // Workaround: compile with the default (sm_100/v9.1), then patch the
        // PTX header to sm_86/v8.0. Our kernel uses only standard PTX arithmetic
        // with no Blackwell-specific instructions, so this is safe.
        cuda_builder::CudaBuilder::new(manifest_dir.join("kernels"))
            .copy_to(&ptx_out)
            .build()
            .expect("CUDA kernel compilation failed");

        // Patch PTX header: sm_100/v9.1 → sm_86/v8.0 for Ampere (A6000) compat.
        let ptx = std::fs::read_to_string(&ptx_out).expect("read PTX");
        let ptx = ptx
            .replace(".version 9.1", ".version 8.0")
            .replace(".target sm_100", ".target sm_86");
        std::fs::write(&ptx_out, ptx).expect("write patched PTX");
    }

    #[cfg(not(feature = "cuda"))]
    {
        // Write empty placeholder so include_str! in cuda_backend.rs still compiles.
        std::fs::write(&ptx_out, "").unwrap();
    }
}
