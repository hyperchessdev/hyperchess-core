# CLI reference

The driver binary is named `hyperchess`. Run it during development with:

```bash
cargo run -p hyperchess-driver -- <command>
```

Or build a release binary once and invoke it directly:

```bash
cargo build --release -p hyperchess-driver
./target/release/hyperchess <command>
```

## Commands at a glance

| Command | Purpose |
| --- | --- |
| `show` | Print the starting position |
| `perft [DEPTH]` | Count legal move-tree nodes from the starting position (correctness check) |
| `play` | Run one or many engine-vs-engine games and export the results |
| `gpu-info` | Detect and report CUDA/GPU availability |
| `bench-eval` | Benchmark GPU vs. CPU batch evaluation (only built with `--features cuda`) |
| `uci` | Start the native UCI protocol server — see [UCI integration](UCI.md) |
| `api` | Start the stateless REST/OpenAPI service |

```bash
cargo run -p hyperchess-driver -- show
cargo run -p hyperchess-driver -- perft 3
cargo run -p hyperchess-driver -- gpu-info
cargo run -p hyperchess-driver -- uci
cargo run -p hyperchess-driver -- api
```

## Engine names

Every engine-selecting flag (`--white`/`--black` in `play`, `algorithm` in the REST API)
accepts one of these names. Several have short aliases, kept for backward compatibility with
earlier internal naming:

| Canonical name | Aliases | What it runs |
| --- | --- | --- |
| `random` | — | Uniformly random legal move — a baseline, not a real engine |
| `alphabeta` | `ab` | Fixed-depth alpha-beta with the always-on technique set — see [search architecture](search-architecture.md) |
| `iterative` | `id` | Iterative-deepening alpha-beta |
| `guided` | `guided_ab` | Alpha-beta using expensive, eval-guided move ordering instead of static ordering |
| `guided_id` | — | Guided ordering, iterative-deepening variant |
| `strategic` | `strategic_like` | `TimedSearcher` running the **Strategic** profile |
| `aggressive` | `commercial`, `stockfish_like` | `TimedSearcher` running the **Aggressive** profile — Strategic plus the full speculative pruning family |
| `mcts` | — | CPU Monte Carlo Tree Search (UCT) |
| `cuda_mcts` | — | GPU-batched MCTS — requires a `--features cuda` build |

`strategic`/`aggressive` are the current names after a rename for trademark safety; the retired
aliases (`stockfish_like`, `commercial`) still resolve, so older scripts and datasets that
reference them keep working. See [search architecture: Profiles](search-architecture.md#profiles)
for what actually differs between Balanced, Strategic, and Aggressive.

## Play and export a game

```bash
cargo run -p hyperchess-driver -- play \
  --white alphabeta --black aggressive \
  --depth 4 --random-seed 42 \
  --out-dir ./games
```

### Full flag reference (`play`)

| Flag | Default | Meaning |
| --- | --- | --- |
| `--white`, `--black` | `cuda_mcts` | Engine name for each side (see table above) |
| `--depth` | `4` | Alpha-beta-family search depth for both sides |
| `--white-depth`, `--black-depth` | (unset) | Per-side depth override; falls back to `--depth` |
| `--simulations` | `800` | MCTS simulations per move for both sides |
| `--white-simulations`, `--black-simulations` | (unset) | Per-side simulation-count override |
| `--batch-size` | `1024` | GPU batch size for `cuda_mcts` (leaves evaluated per kernel launch) |
| `--max-moves` | `224` | Half-move cap before adjudicating a draw by move limit (224 half-moves = 112 full moves, matching the [scaled 50-move rule](hyperchess-laws.md#5-draws)) |
| `--threads` | `0` | Parallel worker threads; `0` = auto-detect CPU cores |
| `--out-dir` | `./games` | Output directory for exported game files |
| `--games` | `1` | Number of games to play; `>1` enables parallel dataset-generation mode |
| `--format` | `default` | `default` or `nnue-plain` (adds a training-oriented plain-text export) |
| `--random-seed` | `0` | RNG seed; `0` auto-generates one from OS entropy. Fix this for reproducibility |
| `--random-opening-plies` | `2` | Uniformly-random opening plies played before the engines take over — vary per worker to diversify a dataset |
| `--white-skill`, `--black-skill` | (unset) | Skill level 1–20; maps internally to a depth/simulation-count pair, recorded in game logs, purely informational |
| `--move-timeout-secs` | `0` (no timeout) | If a search exceeds this, retry at a weaker configuration rather than abandon the move |
| `--progress-file` | (unset) | Path to a JSON progress snapshot, rewritten atomically after each completed game |

`--games N` with `N > 1` parallelizes across independent games (one OS thread per game, up to
`--threads`), not within a single game's search. Use a fixed `--random-seed` for a fully
reproducible single game, and a distinct seed per worker (plus varied
`--random-opening-plies`) when generating a diverse multi-game dataset. Because the engine's own
XorShift64* PRNG is seeded, MCTS runs are exactly reproducible under a fixed seed too — see
[search architecture: MCTS](search-architecture.md#mcts-srcmctsrs).

Each completed game writes: human-readable statistics, a JSON summary, an HSAN game record, and
HFEN position-by-position export. `--format nnue-plain` additionally appends to a shared
`training.plain` file suited to neural-network training pipelines.

## Local REST API

```bash
cargo run -p hyperchess-driver -- api
curl http://localhost:8080/health
```

Binds to `HOST:PORT` — default `0.0.0.0:8080`, both overridable by environment variable.
`ENGINE_DEFAULT_DEPTH` (default `4`) and `ENGINE_THREADS` set process-wide defaults; nothing
else is required to boot — no database, no auth service, no config file.

| Route | Method | Purpose |
| --- | --- | --- |
| `/health` | GET | Liveness check |
| `/board/fen-validate` | POST | Validate an HFEN string |
| `/move/legal` | POST | List legal moves (plain UCI) for a given position |
| `/move/best` | POST | Ask the engine for its best move + evaluation |
| `/docs` | GET | Interactive Swagger UI |
| `/openapi.json` | GET | Raw OpenAPI schema |

`POST /move/best` accepts `fen` (required), plus optional `algorithm` (any name from the
[engine names](#engine-names) table, default `alphabeta`), `depth` (default
`ENGINE_DEFAULT_DEPTH`, itself defaulting to `4`), and `simulations` (default `800`, used only
by MCTS-family algorithms). It returns `best_move` (plain UCI) and `eval_cp` — the static
evaluation, in centipawns from the side-to-move's perspective, of the position before the move.
Every request is self-contained and stateless — see
[SECURITY.md](../SECURITY.md#scope--what-counts-as-a-vulnerability-here) for the statelessness
guarantee this depends on.

## Perft as a correctness tool

```bash
cargo run -p hyperchess-driver -- perft 4
```

`perft` counts leaf nodes of the legal move tree from the starting position at a given depth —
the standard chess-engine correctness check, not a measure of playing strength. A `perft`
mismatch almost always means a move-generation bug. Report one with the exact depth, the
expected count if known, and your engine commit SHA.
