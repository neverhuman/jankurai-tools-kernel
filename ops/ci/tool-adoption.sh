#!/usr/bin/env bash
# Tool-adoption evidence lane.
#
# jankurai replaces a fleet of ad-hoc tools (manual scoring, gitleaks-only
# security, hand-rolled coverage/contract drift checks) with first-class
# subcommands. This lane runs each adopted command in CI and writes its
# evidence artifact under target/jankurai/ so the audit can prove the
# replacement actually executed. The matching artifacts are uploaded by the
# workflow's actions/upload-artifact step.
set -euo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"
cd "$REPO_ROOT"

mkdir -p target/jankurai target/jankurai/security target/jankurai/coverage \
         target/jankurai/proofbind target/jankurai/proofmark
cp agent/baselines/main.repo-score.json target/jankurai/accepted-baseline.json
compiled_auditor_version="$(jankurai --version | awk '{print $NF}')"
proofbind_base="${JANKURAI_PROOFBIND_BASE:-origin/main}"
git rev-parse --verify "${proofbind_base}^{commit}" >/dev/null

# proofbind: changed-surface proof obligation routing.
log "tool-adoption: proofbind verify"
jankurai proofbind verify . --changed-from "$proofbind_base"
# Adopted artifacts: target/jankurai/proofbind/surface-witness.json
# target/jankurai/proofbind/obligations.json

# proofmark-rust: in-diff mutation and coverage witness for Rust.
log "tool-adoption: proofmark rust"
jankurai proofmark rust . --obligations target/jankurai/proofbind/obligations.json
jq -e --arg auditor_version "$compiled_auditor_version" \
  '.auditor_version == $auditor_version' \
  target/jankurai/proofmark/proof-receipt.json >/dev/null
jankurai proofbind verify . --changed-from "$proofbind_base" --mode required \
  --proof-receipts target/jankurai/proofmark
# Adopted artifacts: target/jankurai/proofmark/proofmark-receipt.json
# target/jankurai/proofmark/proof-receipt.json

# copy-code: duplication triage replacing ad-hoc copy-code review.
log "tool-adoption: copy-code"
jankurai copy-code . --json target/jankurai/copy-code.json --md target/jankurai/copy-code.md
# Adopted artifacts: target/jankurai/copy-code.json target/jankurai/copy-code.md

# security: secret + dependency + SBOM/provenance evidence in one lane.
log "tool-adoption: security run"
jankurai security run . --out target/jankurai/security/evidence.json --script ops/ci/security.sh --profile release --strict
# Adopted artifact: target/jankurai/security/evidence.json

# ci/git/release bad-behavior: language-level workflow safety tests.
log "tool-adoption: language bad-behavior tests"
cargo test --workspace --locked language_rules -- --nocapture 2>&1 | tee target/jankurai/language-bad-behavior.log
# Adopted artifact: target/jankurai/language-bad-behavior.log

# vibe-coverage: tips-backed vibe coverage replacing manual review.
log "tool-adoption: vibe coverage"
jankurai vibe coverage --source agent/vibe-coverage.toml --tips tips/vibe_coding --json target/jankurai/vibe-coverage.json --md target/jankurai/vibe-coverage.md
# Adopted artifacts: target/jankurai/vibe-coverage.json target/jankurai/vibe-coverage.md

# coverage-evidence: coverage audit replacing line-only coverage gates.
log "tool-adoption: coverage audit"
jankurai coverage audit . --config agent/coverage-sources.toml --json target/jankurai/coverage/coverage-audit.json --md target/jankurai/coverage/coverage-audit.md
# Adopted artifacts: target/jankurai/coverage/coverage-audit.json
# target/jankurai/coverage/coverage-audit.md

# audit-ci / proof-routing / contract-drift / authz-matrix / agent-tool-supply
# / release-readiness / cost-budget all adopt the final ratchet audit command.
log "tool-adoption: final ratchet audit"
jankurai audit . --mode ratchet --baseline target/jankurai/accepted-baseline.json --json target/jankurai/repo-score.json --md target/jankurai/repo-score.md --no-score-history --repair-queue-jsonl target/jankurai/repair-queue.jsonl --full
jq -e --arg auditor_version "$compiled_auditor_version" \
  '.decision.passed == true and .score >= 85 and (.caps_applied | length) == 0 and .decision.hard_findings == 0 and .auditor_version == $auditor_version' \
  target/jankurai/repo-score.json >/dev/null
# Adopted artifacts: .jankurai/repo-score.json .jankurai/repo-score.md
# target/jankurai/repair-queue.jsonl
