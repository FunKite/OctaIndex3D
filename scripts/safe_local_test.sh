#!/usr/bin/env bash
set -euo pipefail

SLOW_TEST="layers::esdf::tests::test_esdf_from_tsdf"
SKIP_SLOW=1
OFFLINE=1
RUN_ADVISORIES=1

usage() {
  cat <<'EOF'
Safer local test runner for OctaIndex3D.

Usage:
  scripts/safe_local_test.sh [options]

Options:
  --include-slow      Include known long-running ESDF test.
  --allow-network     Disable offline mode for cargo.
  --skip-advisories   Skip `cargo deny check advisories`.
  -h, --help          Show this help.
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --include-slow)
      SKIP_SLOW=0
      ;;
    --allow-network)
      OFFLINE=0
      ;;
    --skip-advisories)
      RUN_ADVISORIES=0
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "Unknown option: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
  shift
done

if [[ ! -f Cargo.toml || ! -f Cargo.lock ]]; then
  echo "Run this script from the repository root." >&2
  exit 1
fi

echo "==> Preflight: dependency diffs"
if git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
  dep_changes="$(git diff --name-only HEAD -- Cargo.toml Cargo.lock)"
  if [[ -n "${dep_changes}" ]]; then
    echo "Dependency files changed in working tree:" >&2
    echo "${dep_changes}" >&2
    echo "Review dependency diffs before trusting local execution." >&2
  else
    echo "No local dependency file changes detected."
  fi
fi

echo "==> Preflight: scrub common credential env vars"
unset GITHUB_TOKEN GH_TOKEN CARGO_REGISTRIES_CRATES_IO_TOKEN
unset AWS_ACCESS_KEY_ID AWS_SECRET_ACCESS_KEY AWS_SESSION_TOKEN
unset OPENAI_API_KEY ANTHROPIC_API_KEY

# Prefer existing cargo home when writable; otherwise fall back to a local cache.
default_cargo_home="${CARGO_HOME:-${HOME}/.cargo}"
if mkdir -p "${default_cargo_home}" 2>/dev/null; then
  export CARGO_HOME="${default_cargo_home}"
else
  export CARGO_HOME="${PWD}/.cargo-local"
  mkdir -p "${CARGO_HOME}"
fi

test_flags=(--locked)
if [[ ${OFFLINE} -eq 1 ]]; then
  export CARGO_NET_OFFLINE=true
  test_flags+=(--offline)
  echo "Offline mode enabled (set --allow-network to disable)."
fi

if [[ ${RUN_ADVISORIES} -eq 1 ]]; then
  if command -v cargo-deny >/dev/null 2>&1; then
    echo "==> Security check: cargo deny advisories"
    if ! cargo deny check advisories; then
      if [[ ${OFFLINE} -eq 1 ]]; then
        echo "Warning: advisory check failed in offline mode. Re-run with --allow-network to refresh index." >&2
      else
        exit 1
      fi
    fi
  else
    echo "Skipping advisories check (cargo-deny not installed)."
  fi
fi

echo "==> Running tests"
if [[ ${SKIP_SLOW} -eq 1 ]]; then
  cargo test "${test_flags[@]}" -- --skip "${SLOW_TEST}"
else
  cargo test "${test_flags[@]}"
fi

echo "==> Safe local test run complete"
