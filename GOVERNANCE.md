# Governance

HyperChess Core currently uses a lightweight **BDFL** model: the founder makes final decisions
so a small project can move quickly, while discussion and rationale stay public. This is a
documented starting point, not a claim that one person should control a mature commons forever
— see [Evolution](#evolution).

## How decisions are made

| Change type | Process |
| --- | --- |
| Bug fixes, routine maintenance, dependency bumps | Maintainer review and merge |
| New features that don't touch rules, notation, or the public API | Issue/PR discussion, maintainer approval |
| Rules, notation (HFEN/HSAN), search safety invariants, licensing, or security-policy changes | Public design note, comment period, documented decision, tests, release notes |
| Conduct and security reports | Handled privately per [`CODE_OF_CONDUCT.md`](CODE_OF_CONDUCT.md) / [`SECURITY.md`](SECURITY.md) |

Search changes specifically must preserve the safety invariants listed at the end of
[`docs/search-architecture.md`](docs/search-architecture.md) — this isn't a courtesy, CI
enforces most of them.

## RFC / design-note process

We don't run a heavyweight RFC process for a project this size. Instead: open a GitHub
Discussion describing the problem, the proposed change, and its impact on HFEN/HSAN or the
public API if any; leave it open at least a week for a rules/format/licensing change; a
maintainer posts a documented accept/reject/revise decision with rationale before implementation
proceeds as a normal PR.

## Role ladder

```
Contributor → Committer (subsystem merge rights) → Maintainer → Steering (future)
```

Advancement reflects sustained, well-reviewed work, clear communication, and demonstrated care
for correctness and provenance — not a fixed contribution count or tenure. See
[`MAINTAINERS.md`](MAINTAINERS.md) for who holds which role today.

## Release cadence

Time-boxed and predictable rather than continuous — see [`CHANGELOG.md`](CHANGELOG.md) for
what has shipped. Every release credits every contributor by name.

## Values this governance model protects

- **Transparent rationale over false consensus** — decisions are explained, not just announced.
- **Provenance over convenience** — a change that can't be attributed, tested, and explained
  doesn't merge, however it was authored. See [`docs/ATTRIBUTION-AND-TRUST.md`](docs/ATTRIBUTION-AND-TRUST.md).
- **No AI-slop shortcuts** — unaccountable AI-generated submissions are treated like any other
  unaccountable submission: not merged.

## Evolution

As the contributor base grows past a single founder plus occasional PRs, this document will be
revised toward a documented multi-maintainer or Steering model. Any such change goes through the
[design-note process](#rfc--design-note-process) above, publicly, before it takes effect.
