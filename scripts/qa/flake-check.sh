#!/usr/bin/env bash
# flake-check.sh — rerun a crate's test suite many times to catch tests that
# only fail when another test interleaves with them (shared ports, seed
# files, env). A single green CI run misses those.
#
#   scripts/qa/flake-check.sh [--runs N] [--package CRATE] [FILTER]
#
# Default: 10 rounds of `oceanln-httpd` (the suite that binds sockets and
# writes seed files), each run three ways: default threads, --test-threads 32
# (widens interleavings) and --test-threads 1 (order-only bugs). Prints each
# failing test once per failed run; exit status = failed runs (max 255).
set -euo pipefail
RUNS=10; PACKAGE=oceanln-httpd; FILTER=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    --runs) RUNS="$2"; shift 2 ;;
    --package) PACKAGE="$2"; shift 2 ;;
    -h|--help) sed -n '2,12p' "$0"; exit 0 ;;
    *) FILTER="$1"; shift ;;
  esac
done
cd "$(dirname "${BASH_SOURCE[0]}")/../.."
if ! build="$(cargo test -q -p "$PACKAGE" --no-run 2>&1)"; then echo "$build"; exit 255; fi

failed=0; total=0
for mode in default 32 1; do
  threads=(); [[ "$mode" != default ]] && threads=(--test-threads "$mode")
  mode_failed=0
  for ((i = 1; i <= RUNS; i++)); do
    total=$((total + 1))
    if ! out="$(cargo test -q -p "$PACKAGE" "$FILTER" -- ${threads[@]+"${threads[@]}"} 2>&1)"; then
      failed=$((failed + 1)); mode_failed=$((mode_failed + 1))
      grep -E -- "--- FAILED|^test .* FAILED" <<<"$out" | sed "s/^/  [$mode #$i] /" || true
    fi
  done
  echo "threads=$mode: $((RUNS - mode_failed))/$RUNS runs passed"
done
echo "flake-check: $PACKAGE ${FILTER:-(all)} — $failed of $total runs failed"
exit $((failed > 255 ? 255 : failed))
