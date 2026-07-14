#!/usr/bin/env bash
# Security lane: delegate to the canonical release-grade security wrapper.
set -euo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"
cd "$REPO_ROOT"

log "security lane: canonical tools/security-lane.sh"
bash tools/security-lane.sh
