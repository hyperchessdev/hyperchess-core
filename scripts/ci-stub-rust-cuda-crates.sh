#!/usr/bin/env bash
# Seeds minimal stub manifests for the local rust-cuda crates that hyperchess-driver and
# hyperchess-search-cuda reference via absolute local `path` dependencies
# (/projects/github/rust-cuda/crates/{cust,cust_raw,cuda_builder} — see §5 of
# docs/hyperchess-core-extraction-plan.md). This sandbox happens to have a real rust-cuda
# checkout there; a fresh CI runner clone does not.
#
# Cargo must structurally resolve every path dependency's manifest (name, version, and every
# feature referenced by the depending crate's Cargo.toml) to build its resolver graph, even when
# the optional feature that pulls the dependency in is never activated. Discovered this,
# and the exact feature requirements below, via real triggered GitLab CI pipeline runs — each
# missing piece produced a distinct, specific Cargo resolver error, not a generic failure:
#   - "failed to read .../Cargo.toml: No such file or directory"  -> crate didn't exist at all
#   - "does not have that feature"                                -> crate existed, feature didn't
# None of these stubs are ever actually compiled (nothing in this CI pipeline builds with
# --features cuda), so their contents are irrelevant beyond satisfying manifest resolution.
set -euo pipefail

# Where to create the stub checkout. Default matches the dev-machine layout;
# GitHub CI overrides it on Windows, where cargo resolves the unix-style
# absolute path against the working drive.
STUB_ROOT="${STUB_ROOT:-/projects/github/rust-cuda}"

declare -A FEATURES=(
  [cust]=""
  [cust_raw]="driver"
  [cuda_builder]="rustc_codegen_nvvm,llvm19"
)
declare -A VERSIONS=(
  [cust]="0.3.2"
  [cust_raw]="0.11.3"
  [cuda_builder]="0.3.0"
)

for name in "${!VERSIONS[@]}"; do
  dir="${STUB_ROOT}/crates/${name}"
  mkdir -p "${dir}/src"
  {
    printf '[package]\nname = "%s"\nversion = "%s"\nedition = "2021"\n' "$name" "${VERSIONS[$name]}"
    if [ -n "${FEATURES[$name]}" ]; then
      printf '\n[features]\n'
      IFS=',' read -ra feats <<< "${FEATURES[$name]}"
      for f in "${feats[@]}"; do
        printf '%s = []\n' "$f"
      done
    fi
  } > "${dir}/Cargo.toml"
  # Newline-terminated comment: `cargo fmt --all` formats path dependencies
  # too, and a zero-byte lib.rs fails `--check` (rustfmt adds a newline).
  printf '// rust-cuda stub for CI manifest resolution; never compiled.\n' > "${dir}/src/lib.rs"
done
