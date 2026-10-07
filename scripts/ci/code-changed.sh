#!/usr/bin/env bash
# code-changed.sh — does a change touch anything besides documentation?
#
#   scripts/ci/code-changed.sh <base> [<head>]    # prints "true" or "false"
#
# Prints "false" only when every file changed between <base> and <head> is
# documentation: Markdown anywhere, anything under docs/, or LICENSE. Any
# other file prints "true". So does a base that cannot be used (empty, the
# all-zero SHA of a new branch, or not in the local history), so a manual,
# scheduled or first run tests everything. The changed files go to stderr.
set -euo pipefail
base="${1:-}"
head="${2:-HEAD}"

if [[ -z "$base" || "$base" =~ ^0+$ ]] || ! git rev-parse --verify -q "${base}^{commit}" >/dev/null; then
  echo "code-changed: no usable base (${base:-none}); testing everything" >&2
  echo true
  exit 0
fi

files="$(git diff --name-only "$base" "$head")"
if [[ -z "$files" ]]; then
  echo "code-changed: no files changed; testing everything" >&2
  echo true
  exit 0
fi

code=false
while IFS= read -r f; do
  case "$f" in
    *.md | docs/* | LICENSE) echo "  docs  $f" >&2 ;;
    *) echo "  code  $f" >&2; code=true ;;
  esac
done <<<"$files"
echo "$code"
