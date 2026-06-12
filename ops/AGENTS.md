# ops/ Agent Instructions

This cell owns the pinned CI lane scripts and Git hook entrypoints for
jankurai-tools-kernel.

- `ops/ci/<lane>.sh` are the canonical lane scripts. The Justfile and
  `.github/workflows/ci.yml` both delegate here so local and CI runs stay
  identical. Edit the lane script, not the workflow, when changing a lane.
- `ops/ci/lib.sh` holds the shared tool-version pins and artifact assertions.
- `ops/git-hooks/pre-push` runs `bash ops/ci/quality-gates.sh`. Wire it once with
  `git config core.hooksPath ops/git-hooks`.
- Proof lane: the security lane and workflow lint (`bash scripts/ci-local.sh
  security`). Run it before changing anything under `ops/`.
- Owner: ops. Keep every third-party GitHub Action pinned to a 40-character
  commit SHA.
