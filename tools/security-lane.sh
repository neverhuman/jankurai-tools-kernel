#!/usr/bin/env bash
# Canonical release-grade security wrapper for jankurai-tools-kernel.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$REPO_ROOT/ops/ci/lib.sh"
cd "$REPO_ROOT"
mkdir -p target/jankurai/security

emit_step() {
  local label="$1"
  local command="$2"
  local tool="$3"
  local status="$4"
  local exit_code="$5"
  printf 'jankurai-security-step={"label":"%s","shell_command":"%s","tool":"%s","status":"%s","advisory":false,"exit_code":%s}\n' \
    "$label" "$command" "$tool" "$status" "$exit_code"
}

run_step() {
  local label="$1"
  local tool="$2"
  local command="$3"
  shift 3
  log "security: $label"
  set +e
  "$@"
  local exit_code=$?
  set -e
  local status="ran"
  if [ "$exit_code" -ne 0 ]; then
    status="failed"
  fi
  emit_step "$label" "$command" "$tool" "$status" "$exit_code"
  return "$exit_code"
}

npm_audit_or_no_surface() {
  if [ -f package.json ] || [ -f package-lock.json ] || [ -f npm-shrinkwrap.json ]; then
    npm audit --audit-level=high --offline
  else
    log "security: npm audit not applicable (no npm package surface)"
    npm --version >/dev/null
  fi
}

run_step "gitleaks" "gitleaks" \
  "gitleaks detect --source . --no-banner --redact" \
  gitleaks detect --source . --no-banner --redact
run_step "cargo-audit" "cargo-audit" \
  "cargo audit" \
  cargo audit
run_step "npm" "npm" \
  "npm audit --audit-level=high --offline (or verify no npm package surface)" \
  npm_audit_or_no_surface
run_step "zizmor" "zizmor" \
  "zizmor --offline --min-severity high .github/workflows" \
  zizmor --offline --min-severity high .github/workflows
run_step "syft" "syft" \
  "syft scan dir:. --exclude ./target/** -o spdx-json=target/jankurai/security/sbom.spdx.json" \
  syft scan dir:. --exclude './target/**' \
    -o spdx-json=target/jankurai/security/sbom.spdx.json
run_step "cargo-deny" "cargo-deny" \
  "cargo deny check advisories --disable-fetch" \
  cargo deny check advisories --disable-fetch
run_step "grype" "grype" \
  "grype sbom:target/jankurai/security/sbom.spdx.json --fail-on high" \
  grype sbom:target/jankurai/security/sbom.spdx.json --fail-on high
