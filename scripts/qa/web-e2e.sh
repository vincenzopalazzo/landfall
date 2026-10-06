#!/usr/bin/env bash
# web-e2e.sh — registry scenarios QA-201…QA-207: drive the Svelte wizard in a
# headless Chromium (Playwright) against a real `oceanln-httpd --mock-wallet`,
# then verify the signature the wizard showed with `oceanln verify`.
#
#   scripts/qa/web-e2e.sh                 # build dist-qa, run every spec
#   scripts/qa/web-e2e.sh create          # one spec (create | import | recover)
#
# Requirements: node 22 + `npm ci` in oceanln-web, a Chromium Playwright can
# use (`npx playwright install chromium`, or OCEANLN_QA_CHROMIUM=/path/to/chrome
# when one is pre-installed), and oceanln-httpd built with `--features qa-mock`.
#
# Each spec gets a FRESH httpd + seed file, so `create` and `import` cannot
# influence each other. Exit status = failed specs (+1 if the CLI rejects the
# wizard's signature).
set -euo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

HTTPD="$(qa_resolve_bin OCEANLN_HTTPD oceanln-httpd -p oceanln-httpd --features oceanln-httpd/qa-mock)"
BIN="$(qa_resolve_bin OCEANLN oceanln -p oceanln-cli)"
"$HTTPD" --help 2>/dev/null | grep -q -- '--mock-wallet' || {
  echo "error: $HTTPD lacks --mock-wallet; build with: cargo build --release -p oceanln-httpd --features oceanln-httpd/qa-mock" >&2
  exit 2
}
WEB="$QA_ROOT/oceanln-web"
W="$QA_HOME/web"; rm -rf "$W"; mkdir -p "$W"
TOKEN="qa-web-token-$RANDOM$RANDOM"
HTTPD_PORT="$(qa_free_port)"
HTTPD_BASE="http://127.0.0.1:$HTTPD_PORT"
WEB_PORT="${OCEANLN_QA_WEB_PORT:-4173}"
WEB_ORIGIN="http://localhost:$WEB_PORT"
SPECS=("${@:-create import recover}")
[[ $# -eq 0 ]] && SPECS=(create import recover)

# Pre-installed Chromium (e.g. /opt/pw-browsers) when the caller did not say.
if [[ -z "${OCEANLN_QA_CHROMIUM:-}" && -n "${PLAYWRIGHT_BROWSERS_PATH:-}" && -x "$PLAYWRIGHT_BROWSERS_PATH/chromium" ]]; then
  export OCEANLN_QA_CHROMIUM="$PLAYWRIGHT_BROWSERS_PATH/chromium"
fi

PIDS=()
cleanup() { for p in "${PIDS[@]:-}"; do [[ -n "$p" ]] && kill "$p" 2>/dev/null || true; done; }
trap cleanup EXIT INT TERM

# 1. A QA-only bundle with the httpd base + token baked in (never the real
#    `dist/`): the wizard then boots already pointed at our server.
echo "web-e2e: building dist-qa against $HTTPD_BASE"
(cd "$WEB" && VITE_OCEANLN_BASE="$HTTPD_BASE" VITE_OCEANLN_TOKEN="$TOKEN" npx vite build --outDir dist-qa --emptyOutDir >"$W/build.log" 2>&1) || { cat "$W/build.log"; exit 2; }

# 2. Static preview on a fixed port (its origin must be allow-listed by httpd).
(cd "$WEB" && npx vite preview --outDir dist-qa --port "$WEB_PORT" --strictPort >"$W/preview.log" 2>&1) &
PIDS+=($!)
qa_wait_http "$WEB_ORIGIN/" 30

FAILS=0
for spec in "${SPECS[@]}"; do
  seed="$W/seed-$spec"; rm -f "$seed" "$seed.offer"
  # `recover` (QA-210) starts from a server that already holds a seed but
  # never reached the offer step: a closed tab before the backup.
  if [[ "$spec" == recover ]]; then printf '%s\n' "$QA_TEST_MNEMONIC" >"$seed"; chmod 600 "$seed"; fi
  "$HTTPD" --bind "127.0.0.1:$HTTPD_PORT" --seed-file "$seed" --token "$TOKEN" --allow-origin "$WEB_ORIGIN" --mock-wallet >"$W/httpd-$spec.log" 2>&1 &
  hp=$!; PIDS+=($hp)
  qa_wait_http "$HTTPD_BASE/health" 30
  echo "── spec: $spec (httpd pid $hp, seed $seed)"
  if (cd "$WEB" && OCEANLN_QA_WEB="$WEB_ORIGIN" OCEANLN_QA_OUT="$W/$spec.json" npx playwright test "e2e/$spec.e2e.ts"); then
    qa_pass "QA-2xx $spec spec"
  else
    qa_fail "QA-2xx $spec spec"; FAILS=$((FAILS + 1))
  fi
  kill "$hp" 2>/dev/null || true; wait "$hp" 2>/dev/null || true
done

# 3. The wizard's signature must satisfy the same check OCEAN runs.
if [[ -f "$W/create.json" ]]; then
  addr="$(qa_json address <"$W/create.json")"; msg="$(qa_json message <"$W/create.json")"; sig="$(qa_json signature <"$W/create.json")"
  qa_check QA-208 "oceanln verify accepts the signature the wizard displayed ($addr)" "$BIN" verify --address "$addr" --message "$msg" --signature "$sig"
  # And the phrase the wizard showed really controls that address.
  phrase="$(python3 -c 'import json,sys;print(" ".join(json.load(open(sys.argv[1]))["phrase"]))' "$W/create.json")"
  qa_check QA-209 "the phrase the wizard revealed derives the address it displayed (init --dry-run)" bash -c '
    [[ "$(echo "$1" | XDG_CONFIG_HOME="$4/xdg" HOME="$4/home" "$2" init --dry-run --json 2>/dev/null | python3 -c "import sys,json;print(json.load(sys.stdin)[\"mining_address\"])")" == "$3" ]]' _ "$phrase" "$BIN" "$addr" "$W"
fi

qa_summary web-e2e
