# jankurai-tools-kernel

<!-- jankurai-badge:start -->
[![Jankurai score: 89/100](agent/jankurai-badge.svg)](agent/jankurai-badge.json)
<!-- jankurai-badge:end -->

Shared audit substrate for the **jankurai** auditor and the Jankurai standard.
This repository ships one crate, `jankurai-audit-kernel` (model, scan, rules,
caps, boundaries, validation, render), extracted from `jankurai-core` so the
core CLI and sibling tools depend on one kernel. It is one member of the Jankurai
split family; read [`SPLIT.md`](SPLIT.md) for the family contract and
[`AGENTS.md`](AGENTS.md) for agent routing rules.

## Stack

Rust core + TypeScript/React/Vite product surface + PostgreSQL truth + generated
contracts + exception-only Python AI/data service. New implementation is
Rust-first; see [`docs/architecture.md`](docs/architecture.md).

## Quick start

```bash
# One-command setup (toolchain + locked dependencies).
just setup

# Deterministic fast lane (check + tests).
just fast

# Full local check: format, lint, fast, security, and self-audit.
just check
```

The full command surface lives in the root [`Justfile`](Justfile). Continuous
integration runs the same lanes under
[`.github/workflows/ci.yml`](.github/workflows/ci.yml).

## Layout

| Path | Role |
| --- | --- |
| `crates/jankurai-audit-kernel` | shared Rust audit substrate (library) |
| `schemas/` | JSON Schemas for the auditor's artifacts |
| `agent/` | machine-readable owner, test, boundary, and proof maps |
| `docs/` | architecture, testing, boundaries, release, and exception docs |
| `ops/` | pinned CI script entrypoints |
| `scripts/` | local CI helpers |

## Documentation

- [Architecture](docs/architecture.md)
- [Testing](docs/testing.md)
- [Boundaries](docs/boundaries.md)
- [Release process](docs/release.md)
- [Agent exceptions and overrides](docs/exceptions.md)

## Versioning

The current version is recorded in [`VERSION`](VERSION) and the change history in
[`CHANGELOG.md`](CHANGELOG.md). Release mechanics are documented in
[`docs/release.md`](docs/release.md).

## License

See [`LICENSE`](LICENSE).
