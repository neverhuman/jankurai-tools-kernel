# jankurai-tools-kernel Agent Instructions

## Zero-new-worktree policy

- Never run `git worktree add` or `git worktree move` anywhere, including under `/tmp`. Work only
  in the claimed canonical primary checkout. If it is dirty, busy, or held, wait for a stopped-head
  handoff.
- Existing worktrees are cleanup inputs only. Never force-remove or prune them. Any removal needs
  separate authority, regression/salvage proof, and recursive proof that the directory contains no
  symlink.
- Exact-SHA CI isolation may use only an automatically removed standalone clone or sandbox that is
  not registered with `git worktree`.

Read `SPLIT.md` first. This repository is one member of the Jankurai split family.

- Canonical local Jeryu repo: `root/jankurai-tools-kernel`.
- Public mirror target: `github.com/neverhuman/jankurai-tools-kernel`.
- Do not add committed cross-repo `path = "../..."` dependencies. Use the hub fusion workspace for local path patches.
- Do not hand-edit generated artifacts listed in `agent/generated-zones.toml`.
- Run `bash scripts/ci-local.sh required` before handing off changes.
