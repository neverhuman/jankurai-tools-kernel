# jankurai-tools-kernel

Status: initial split-family extraction
Owner: Jankurai maintainers
Last reviewed: 2026-06-12
Applies to: jankurai-tools-kernel

## Role

Shared audit substrate (model, scan, rules, caps, boundaries, validation, render)
extracted from jankurai-core so the core CLI and sibling tools depend on one kernel.

## Repositories

- Local authoritative repo: `root/jankurai-tools-kernel`
- Public mirror: `neverhuman/jankurai-tools-kernel`
- Release tag pattern: `jankurai-tools-kernel-v1.7.0-split.2`
- Source extraction commit: `446af94`

## Split Rules

- Jeryu remains authoritative; GitHub is the public mirror.
- Release builds depend on immutable GitHub tags, not branches.
- Local development uses the hub `scripts/fuse.sh` output under `.fusion/`.
- Committed manifests must not depend on sibling checkout paths.
- Generated outputs are regenerated from their source contracts or build commands.

## Required Local Check

```bash
bash scripts/ci-local.sh required
```
