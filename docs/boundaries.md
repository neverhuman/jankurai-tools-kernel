# Boundaries

This repository is a single-purpose Rust workspace: the `jankurai-audit-kernel`
crate, the shared audit substrate for the Jankurai standard. The
machine-readable boundary manifest is
[`agent/boundaries.toml`](../agent/boundaries.toml); this document is its prose
companion.

## Domain

The product domain lives entirely under `crates/jankurai-audit-kernel/src`.
There is no web surface, no PostgreSQL database, and no Python AI/data service
committed in this repo, so those stack arms of the family standard are not
applicable here.

## Forbidden imports in domain code

Pure domain logic under `crates/jankurai-audit-kernel/src/domain` (when present)
must not reach for ambient effects. The forbidden import prefixes are declared in
`agent/boundaries.toml` and include `std::fs`, `std::env`, `std::net`,
`std::time::SystemTime`, and any direct I/O, randomness, or networking crate.
Effects belong in adapter and command layers, not in the domain core.

## Detector-marker strings

The scan and language-rule modules carry queue/streaming, input-boundary, and
dead-language *marker strings* (for example `rdkafka`, `innerhtml`, `owner_id`)
as detection patterns. They are pattern literals, not live hazardous usage. The
documented `[[streaming_exception]]` in `agent/boundaries.toml` records the
brownfield classification for the queue-client markers so the detector source is
not misread as an out-of-boundary streaming client.

## Generated zones

Generated output is never hand-edited. The only generated zone in this repo is
`target/` (Cargo build output), declared in
[`agent/generated-zones.toml`](../agent/generated-zones.toml). It is regenerated
by the build, not committed.

## Ownership and proof

- [`agent/owner-map.json`](../agent/owner-map.json) assigns an owner to every
  top-level path that exists.
- [`agent/test-map.json`](../agent/test-map.json) routes each owned path to a
  deterministic proof command.
- [`agent/proof-lanes.toml`](../agent/proof-lanes.toml) defines the runnable
  lanes; every lane command executes in this repo alone.

## Reclassification

If a future change adds a web, database, or Python surface, update
`agent/boundaries.toml` first to declare the new boundary block, then add the
matching owner, test, and proof entries before landing the code.
