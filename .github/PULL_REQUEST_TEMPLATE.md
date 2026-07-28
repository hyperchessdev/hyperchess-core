# What & why

<!-- What does this PR change, and what problem does it solve? Link issues with #123. -->

# How was it verified?

<!-- Which of these did you run locally? -->

- [ ] `cargo fmt --all --check`
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
- [ ] `cargo test --workspace`
- [ ] `pnpm run build && pnpm run test` (if TypeScript packages are touched)

# For search/eval changes

- [ ] The [safety invariants](../blob/main/docs/search-architecture.md#safety-invariants-the-checklist) are preserved
- [ ] Strength/impact measured (fixed-node self-play or `examples/golden_measure.rs`) — numbers below

<!-- Paste measurement results here -->
