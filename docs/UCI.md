# UCI integration

HyperChess ships a native server implementing the [Universal Chess Interface
(UCI)](https://www.chessprogramming.org/UCI) protocol subset needed to drive a full game,
adapted for a 12×12 board. It's implemented in `hyperchess-driver::uci` (the server in
`server.rs`/`server_util.rs`), alongside a reusable async UCI **client** and connection pool
(`client.rs`/`pool.rs`) for talking to any UCI-speaking engine — including HyperChess's own —
and a piece-value calibration tool (`calibration.rs`) built on top of that client.

## Start the server

```bash
cargo run -p hyperchess-driver -- uci
```

It reads commands from standard input and writes responses to standard output — keep any
diagnostic logging off stdout, since a GUI or harness parses that stream as the protocol itself.

## Minimal handshake

```text
uci
isready
position startpos
go depth 3
quit
```

## Supported commands

| Command | Behavior |
| --- | --- |
| `uci` | Identifies the engine; expect `uciok` |
| `isready` | Synchronization check; expect `readyok` |
| `ucinewgame` | Resets internal game state for a new game |
| `position startpos [moves ...]` | Sets up the starting position, optionally followed by moves |
| `position fen <HFEN> [moves ...]` | Sets up an arbitrary position from an HFEN string, optionally followed by moves |
| `go depth N` | Search to a fixed depth |
| `go movetime N` | Search for `N` milliseconds |
| `go infinite` | Search until `stop` |
| `go perft N` | Run perft to depth `N` instead of a real search |
| `stop` | Stop the current search and report the best move found so far |
| `quit` / `exit` | Terminate the server |

Moves — after `position ... moves`, and in the engine's own `bestmove` output — use coordinate
UCI notation on files `a`–`l` and ranks `1`–`12`, such as `g3g5` or `a11a12q`. Promotion suffix
letters are `q`, `r`, `b`, `n`, `e` (Eagle), and `h` (Hawk). HPGN-I moves may also be supplied
wherever a move is accepted — `M:g3g5` executes identically to `g3g5`; the identity prefix is
optional for execution and valuable only for replay/analysis. See
[`docs/FORMATS.md`](FORMATS.md) before you start serializing games through this interface.

There is currently no support for time-control commands (`wtime`/`btime`/`winc`/`binc`) or
`go nodes` — only `depth`, `movetime`, `infinite`, and `perft` are recognized. Unrecognized `go`
parameters are ignored rather than rejected; use `movetime` for a wall-clock-bounded search in
the meantime.

## Connecting a GUI

Orthodox desktop chess GUIs assume an 8×8 board and orthodox piece set. Treat any such GUI
integration as a genuine variant-adaptation project: board-size, piece-font (Eagle/Hawk need
their own glyphs), and protocol handling (only the command subset above) all typically need
adaptation before a 12×12 game displays and plays correctly.

## Using the UCI client/pool programmatically

`hyperchess-driver::uci::client` provides an async client that speaks this same handshake, and
`uci::pool` provides a connection pool for managing several concurrent engine processes — useful
for running tournaments or batch evaluations against external UCI engines rather than only
against HyperChess's own built-in searchers.
