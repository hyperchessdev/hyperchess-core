# hyperchess-driver

The `hyperchess` binary — one binary, subcommands, not the source repo's two separate
`hyperchess`/`hyperchess-uci` executables (see the extraction plan §12 Phase 4 / §13 for why).

- `hyperchess play` / `perft` / `show` / `gpu-info` / `bench-eval` (`--features cuda`) —
  engine-vs-engine games, PNG board rendering, HFEN-I/HPGN-I/JSON export (`src/cli/`).
- `hyperchess uci` — the native UCI protocol server (`src/uci/server.rs` +
  `server_util.rs`), plus a reusable async UCI *client* (`src/uci/client.rs`) and connection
  pool (`src/uci/pool.rs`) for talking to any UCI-speaking engine, and a piece-value
  calibration tool (`src/uci/calibration.rs`).

Full architecture, extraction history, and roadmap: see the workspace-level
[`docs/hyperchess-core-extraction-plan.md`](../../../docs/hyperchess-core-extraction-plan.md)
(§2 component table, §12 Phase 4).

## `cuda` feature

Off by default — `cargo build` (no flags) gives a fully working CPU-only binary. With
`--features cuda`, pulls in `hyperchess-search-cuda` (and its own `cuda` feature) plus the
`cust` crate directly for `gpu-info`'s device queries. Requires a local `rust-cuda` checkout —
see `hyperchess-search-cuda`'s own docs.
