#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT_DIR}"

base_ref="${1:-}"

if [[ -z "${base_ref}" ]]; then
  if [[ -n "${GITHUB_BASE_REF:-}" ]]; then
    base_ref="origin/${GITHUB_BASE_REF}"
  elif git rev-parse --verify origin/main >/dev/null 2>&1; then
    base_ref="origin/main"
  elif git rev-parse --verify HEAD^ >/dev/null 2>&1; then
    base_ref="HEAD^"
  else
    echo "Skipping changelog guard: no comparison base available."
    exit 0
  fi
fi

if ! git rev-parse --verify "${base_ref}" >/dev/null 2>&1; then
  echo "Skipping changelog guard: comparison base '${base_ref}' is unavailable."
  exit 0
fi

changed_files="$(git diff --name-only "${base_ref}"...HEAD)"

if [[ -z "${changed_files}" ]]; then
  echo "No changes detected relative to ${base_ref}; skipping changelog guard."
  exit 0
fi

requires_changelog=0
while IFS= read -r path; do
  case "${path}" in
    .github/workflows/*|Cargo.toml|Cargo.lock)
      requires_changelog=1
      ;;
  esac
done <<< "${changed_files}"

if [[ "${requires_changelog}" -eq 0 ]]; then
  echo "No workflow or dependency maintenance changes detected."
  exit 0
fi

if grep -Fxq "CHANGELOG.md" <<< "${changed_files}"; then
  echo "Changelog entry detected for workflow/dependency maintenance changes."
  exit 0
fi

echo "Workflow or dependency maintenance changes require a matching CHANGELOG.md update." >&2
echo "Changed files relative to ${base_ref}:" >&2
echo "${changed_files}" >&2
exit 1
