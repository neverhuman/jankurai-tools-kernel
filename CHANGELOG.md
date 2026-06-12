# Changelog

All notable changes to jankurai-tools-kernel are documented in this file. The
format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and
this project adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html). The authoritative
version string lives in [`VERSION`](VERSION).

## [Unreleased]

### Added

- Root `Justfile` command surface with `setup`, `fast`, `check`, `security`, and
  `audit` lanes for one-command setup and validation.
- GitHub Actions CI (`.github/workflows/ci.yml`) with build, security, and
  jankurai audit jobs, all third-party actions pinned to commit SHAs.
- Pinned CI lane scripts under `ops/ci/` and local entrypoints under `scripts/`.
- Agent-readable documentation: `README.md`, `docs/architecture.md`,
  `docs/testing.md`, `docs/boundaries.md`, `docs/release.md`, and
  `docs/exceptions.md`.
- Agent control plane under `agent/`: `audit-policy.toml`, `owner-map.json`,
  `test-map.json`, `boundaries.toml`, `generated-zones.toml`,
  `security-policy.toml`, `tool-adoption.toml`, `coverage-sources.toml`,
  `copy-code-allowlist.toml`, and `proof-lanes.toml`, scoped to this repo.
- `.cargo/config.toml`, `VERSION`, and `rust-toolchain.toml` for hermetic,
  reproducible builds.

## [1.7.0] - 2026-06-12

### Added

- Initial split-family extraction of the shared `jankurai-audit-kernel` crate
  (model, scan, rules, caps, boundaries, validation, render) and its JSON Schema
  contracts from `jankurai-core`.
