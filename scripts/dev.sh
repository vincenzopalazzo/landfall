#!/usr/bin/env bash
# Dev runner: start oceanln-httpd (and the oceanln-web frontend once it exists)
# for local end-to-end development.
#
# Usage:
#   scripts/dev.sh                 # build + run the server (+ web app if present)
#
# Env overrides:
#   OCEANLN_BIND        loopback addr to bind        (default 127.0.0.1:7762)
#   OCEANLN_SEED_FILE   seed file path               (default ./.dev/seed)
#   OCEANLN_TOKEN       bearer token                 (default dev-only fixed token)
#   OCEANLN_WEB_ORIGIN  allowed browser Origin       (default http://localhost:5173)
#
# The seed file is created on first /generate (or import an existing phrase via
# /import). Everything binds loopback only.
set -euo pipefail

cd "$(dirname "$0")/.."

BIND="${OCEANLN_BIND:-127.0.0.1:7762}"
SEED_FILE="${OCEANLN_SEED_FILE:-./.dev/seed}"
# A fixed token is fine for LOCAL dev (loopback only). Do NOT use in production —
# in prod omit --token and the server mints a fresh one.
TOKEN="${OCEANLN_TOKEN:-dev-oceanln-local-token}"
WEB_ORIGIN="${OCEANLN_WEB_ORIGIN:-http://localhost:5173}"

mkdir -p "$(dirname "$SEED_FILE")"

echo "==> building oceanln-httpd"
cargo build -p oceanln-httpd

echo "==> starting oceanln-httpd on http://$BIND"
echo "    seed file:   $SEED_FILE"
echo "    bearer token: $TOKEN"
echo "    allow-origin: $WEB_ORIGIN"
target/debug/oceanln-httpd \
  --bind "$BIND" \
  --seed-file "$SEED_FILE" \
  --token "$TOKEN" \
  --allow-origin "$WEB_ORIGIN" &
SERVER_PID=$!

cleanup() {
  echo
  echo "==> stopping (server pid $SERVER_PID)"
  kill "$SERVER_PID" 2>/dev/null || true
  wait "$SERVER_PID" 2>/dev/null || true
}
trap cleanup EXIT INT TERM

# Give the server a moment, then sanity-check it's up.
sleep 1
if command -v curl >/dev/null 2>&1; then
  echo "==> health: $(curl -fsS "http://$BIND/health" || echo 'NOT READY')"
fi

if [ -f oceanln-web/package.json ]; then
  echo "==> starting oceanln-web (Vite) — token injected via VITE_OCEANLN_TOKEN"
  ( cd oceanln-web
    [ -d node_modules ] || npm install
    VITE_OCEANLN_TOKEN="$TOKEN" VITE_OCEANLN_BASE="http://$BIND" npm run dev )
else
  echo
  echo "oceanln-web/ not found here. Server is running; drive it directly, e.g.:"
  echo "  curl -fsS http://$BIND/health"
  echo "  curl -fsS -X POST http://$BIND/generate -H \"Authorization: Bearer $TOKEN\""
  echo "Press Ctrl-C to stop the server."
  wait "$SERVER_PID"
fi
