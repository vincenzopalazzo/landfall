#!/usr/bin/env bash
# screenshots.sh — regenerate the README's onboarding screenshots.
#
#   scripts/screenshots.sh                # writes docs/screenshots/*.png
#   LANDFALL_SHOTS_DIR=/tmp/shots scripts/screenshots.sh
#
# Drives the real wizard in headless Chromium against a fresh
# `landfall-httpd --mock-wallet` (Lexe stubbed, seed + BIP-322 real), exactly
# like scripts/qa/web-e2e.sh, and saves one PNG per onboarding step
# (landfall-web/e2e/screenshots.e2e.ts). Needs the qa-mock build and a
# Chromium for Playwright (see scripts/qa/README.md).
set -euo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/qa/lib.sh"

HTTPD="$(qa_resolve_bin LANDFALL_HTTPD landfall-httpd -p landfall-httpd --features landfall-httpd/qa-mock)"
"$HTTPD" --help 2>/dev/null | grep -q -- '--mock-wallet' || {
  echo "error: $HTTPD lacks --mock-wallet; build with: cargo build --release -p landfall-httpd --features landfall-httpd/qa-mock" >&2
  exit 2
}
WEB="$QA_ROOT/landfall-web"
OUT="${LANDFALL_SHOTS_DIR:-$QA_ROOT/docs/screenshots}"
W="$QA_HOME/shots"; rm -rf "$W"; mkdir -p "$W" "$OUT"
TOKEN="shots-token-$RANDOM$RANDOM"
HTTPD_PORT="$(qa_free_port)"
HTTPD_BASE="http://127.0.0.1:$HTTPD_PORT"
WEB_PORT="${LANDFALL_QA_WEB_PORT:-4174}"
WEB_ORIGIN="http://localhost:$WEB_PORT"
if [[ -z "${LANDFALL_QA_CHROMIUM:-}" && -n "${PLAYWRIGHT_BROWSERS_PATH:-}" && -x "$PLAYWRIGHT_BROWSERS_PATH/chromium" ]]; then
  export LANDFALL_QA_CHROMIUM="$PLAYWRIGHT_BROWSERS_PATH/chromium"
fi

PIDS=()
cleanup() { for p in "${PIDS[@]:-}"; do [[ -n "$p" ]] && kill "$p" 2>/dev/null || true; done; }
trap cleanup EXIT INT TERM

echo "screenshots: building dist-shots against $HTTPD_BASE"
(cd "$WEB" && VITE_LANDFALL_BASE="$HTTPD_BASE" VITE_LANDFALL_TOKEN="$TOKEN" npx vite build --outDir dist-shots --emptyOutDir >"$W/build.log" 2>&1) || { cat "$W/build.log"; exit 2; }
(cd "$WEB" && npx vite preview --outDir dist-shots --port "$WEB_PORT" --strictPort >"$W/preview.log" 2>&1) &
PIDS+=($!)
qa_wait_http "$WEB_ORIGIN/" 30

"$HTTPD" --bind "127.0.0.1:$HTTPD_PORT" --seed-file "$W/seed" --token "$TOKEN" --allow-origin "$WEB_ORIGIN" --mock-wallet >"$W/httpd.log" 2>&1 &
PIDS+=($!)
qa_wait_http "$HTTPD_BASE/health" 30

(cd "$WEB" && LANDFALL_QA_WEB="$WEB_ORIGIN" LANDFALL_SHOTS_DIR="$OUT" npx playwright test e2e/screenshots.e2e.ts)
rm -rf "$WEB/dist-shots"
ls -la "$OUT"/*.png
