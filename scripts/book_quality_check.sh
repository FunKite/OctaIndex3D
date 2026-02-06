#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT_DIR}"

echo "==> Generating API contract"
./scripts/generate_book_api_contract.sh

echo "==> Checking markdown fence parity"
fence_errors=0
while IFS= read -r f; do
  count="$(awk '/^```/{n++} END{print n+0}' "$f")"
  if [ $((count % 2)) -ne 0 ]; then
    echo "Fence mismatch: $f (count=$count)"
    fence_errors=1
  fi
done < <(rg --files book -g '*.md' | sort)
if [ "${fence_errors}" -ne 0 ]; then
  exit 1
fi

echo "==> Checking deprecated/invalid module paths"
invalid_patterns=(
  'octaindex3d::prelude'
  'octaindex3d::temporal::'
  'octaindex3d::compressed::'
  'octaindex3d::gpu::'
  'octaindex3d::ros2::'
  'CONTAINER_VERSION'
)
for p in "${invalid_patterns[@]}"; do
  if rg -n "${p}" book >/dev/null; then
    echo "Invalid path still present: ${p}"
    rg -n "${p}" book
    exit 1
  fi
done

echo "==> Checking appendix example references"
missing=0
while IFS= read -r path; do
  file="${path#*examples/}"
  file="examples/${file%\`*}"
  if [ ! -f "${file}" ]; then
    echo "Missing example file referenced in appendix: ${file}"
    missing=1
  fi
done < <(rg -n '\*\*Related runnable example:\*\* `examples/' book/appendices/appendix_e_example_code.md)
if [ "${missing}" -ne 0 ]; then
  exit 1
fi

echo "==> Book quality checks passed"
