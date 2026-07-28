# File header convention

Every `.rs` and `.ts` source file in this repository carries a short header block at the top.
This document is the single specification for it.

## Format

```rust
// SPDX-License-Identifier: GPL-3.0-or-later
// HyperChess Core — <crate-or-package-name>
// File: <path from repository root>
// Version: <file version>
// Copyright (c) 2026 HyperChess Developer Team
```

Every crate and package in this repository is licensed `GPL-3.0-or-later` (see
[`README.md#license`](../README.md#license)), so the SPDX line is the same everywhere.

For a Rust `lib.rs`/`mod.rs` that also carries an inner module doc comment (`//! ...`), the
header goes first, as plain `//` comments, immediately followed by the `//!` block:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later
// HyperChess Core — hyperchess-rules
// File: crates/hyperchess-rules/src/lib.rs
// Version: 1.0.0
// Copyright (c) 2026 HyperChess Developer Team

//! HyperChess rules — a 12x12 chess variant with Eagle and Hawk pieces.
//! ...
```

This is syntactically safe: ordinary `//` comments before `//!` inner doc comments or `#![...]`
inner attributes don't affect them — both only need to precede any actual item.

## The `Version` field

A **per-file documentation version**, deliberately independent of the crate's or package's
SemVer in `Cargo.toml`/`package.json`. It exists so a reader can tell, at a glance, whether the
file in front of them has had a substantive rewrite versus a typo fix, without needing
`git blame`.

- New files start at `1.0.0`.
- Bump the **patch** version for a small fix that doesn't change the file's public surface or
  behavior.
- Bump the **minor** version when you add or materially change a public item without breaking
  existing callers.
- Bump the **major** version for a rewrite or a breaking change to the file's public surface.
- Don't bump it for whitespace, comment-only, or formatting-only changes.

This convention is new as of the documentation pass that introduced it — see
[`CHANGELOG.md`](../CHANGELOG.md). It doesn't retroactively claim anything about a file's prior
history; every file starts this scheme at `1.0.0` regardless of how long it existed before the
convention did.

## Why a per-file header at all

Because a source file is frequently read, copied, or vendored independent of its build
manifest — pasted into an issue, viewed on GitHub's file view, or copied into a downstream
project. A self-contained header means the license and provenance travel with the file itself —
the same reasoning long-standing engine projects like Stockfish apply to their own per-file
license headers.

## Applying this convention

Add the header to any new file you create — see the
[PR guidelines](../CONTRIBUTING.md#pull-request-guidelines). Don't remove or alter an existing
file's `Copyright` line; if you believe an attribution is wrong, see
[`docs/ATTRIBUTION-AND-TRUST.md`](ATTRIBUTION-AND-TRUST.md) for how to raise it instead of
silently editing it.
