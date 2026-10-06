#!/usr/bin/env bash
# Live team-demo runner: landfall-httpd + landfall-mcp + landfall-web, FULL flow.
#
# Starts the release landfall-httpd (in-process Lexe SDK — default features, so
# NO separate lexe-sidecar is needed), the read-only landfall-mcp proxy in front
# of it, and the landfall-web Vite frontend, then opens the browser. You then
# drive the whole arc from the UI:
#
#   generate seed -> derive mining address -> PROVISION a Lexe node (live) ->
#   create a BOLT12 offer -> BIP-322 sign the OCEAN message -> dashboard
#   (live balances, activity, receive an invoice, send a payment).
#
# ┌─ READ THIS ──────────────────────────────────────────────────────────────┐
# │ /init provisions a REAL Lexe MAINNET wallet for the seed, and the         │
# │ dashboard's "send" (/pay) moves REAL funds. A fresh-seed run creates a    │
# │ brand-new mainnet wallet — "clean from scratch" is NOT free or reversible.│
# │ Requires network access to Lexe's mainnet backend.                        │
# └────────────────────────────────────────────────────────────────────────-─┘
#
# Usage:
#   scripts/web-demo.sh             # fresh state: provision a NEW node live
#   DEMO_REUSE=1 scripts/web-demo.sh   # reuse the existing demo seed (rehearsals)
#
# Env knobs:
#   DEMO_BIND       loopback addr for httpd   (default 127.0.0.1:7762)
#   DEMO_MCP_BIND   loopback addr for mcp     (default 127.0.0.1:7763)
#   DEMO_WEB_PORT   Vite dev port             (default 5173)
#   DEMO_TOKEN      bearer token              (default a fixed local demo token)
#   DEMO_SEED_FILE  seed file path            (default ./.demo/seed)
#   DEMO_NO_OPEN=1  don't auto-open the browser
#   DEMO_NO_MCP=1   skip the landfall-mcp proxy
set -euo pipefail
cd "$(dirname "$0")/.."

BIND="${DEMO_BIND:-127.0.0.1:7762}"
MCP_BIND="${DEMO_MCP_BIND:-127.0.0.1:7763}"
WEB_PORT="${DEMO_WEB_PORT:-5173}"
# Fixed token is fine for LOCAL loopback dev; the web app needs to know it up
# front to inject into requests. Override with DEMO_TOKEN if you like.
TOKEN="${DEMO_TOKEN:-demo-landfall-local-token}"
SEED_FILE="${DEMO_SEED_FILE:-./.demo/seed}"
HTTPD=target/release/landfall-httpd
MCP=target/release/landfall-mcp

mkdir -p "$(dirname "$SEED_FILE")"

# --- clean-state guard ------------------------------------------------------
# In fresh mode we must start with NO seed so /generate works (it refuses to
# overwrite an existing seed with 409 — a nasty surprise mid-demo). In reuse
# mode we expect the seed to already exist and skip the generate step.
if [ "${DEMO_REUSE:-0}" = "1" ]; then
  if [ ! -f "$SEED_FILE" ]; then
    echo "DEMO_REUSE=1 but no seed at $SEED_FILE — run once without it to provision a node first." >&2
    exit 1
  fi
  echo "==> REUSE mode: reusing the existing demo wallet at $SEED_FILE"
  echo "    (skip the 'generate'/'provision' steps in the UI — go straight to the dashboard)"
else
  if [ -f "$SEED_FILE" ]; then
    cat >&2 <<EOF
==> A seed already exists at $SEED_FILE.
    Fresh mode needs a clean slate (else /generate returns 409 mid-demo).
    Either:
      - rehearse against it:   DEMO_REUSE=1 scripts/web-demo.sh
      - or wipe it and start new: rm $SEED_FILE && scripts/web-demo.sh
        (this orphans the previous mainnet wallet — only do it for throwaways)
EOF
    exit 1
  fi
  echo "==> FRESH mode: you'll create a brand-new Lexe MAINNET wallet live in the UI."
fi

echo "==> building landfall-httpd (release, in-process Lexe SDK)"
cargo build --release -p landfall-httpd
if [ "${DEMO_NO_MCP:-0}" != "1" ]; then
  echo "==> building landfall-mcp (release, read-only proxy)"
  cargo build --release -p landfall-mcp
fi

echo "==> starting landfall-httpd on http://$BIND"
echo "    seed file:    $SEED_FILE"
echo "    bearer token: $TOKEN"
"$HTTPD" \
  --bind "$BIND" \
  --seed-file "$SEED_FILE" \
  --token "$TOKEN" \
  --allow-origin "http://localhost:$WEB_PORT" \
  --allow-origin "http://127.0.0.1:$WEB_PORT" &
SRV=$!

cleanup() {
  echo
  echo "==> stopping (httpd pid $SRV, mcp pid ${MCPPID:-none}, web pid ${WEB:-none})"
  [ -n "${WEB:-}" ] && kill "$WEB" 2>/dev/null || true
  [ -n "${MCPPID:-}" ] && kill "$MCPPID" 2>/dev/null || true
  kill "$SRV" 2>/dev/null || true
  wait 2>/dev/null || true
}
trap cleanup EXIT INT TERM

# Wait for the server to answer /health before bringing up the UI.
for i in $(seq 1 20); do
  if curl -fsS "http://$BIND/health" >/dev/null 2>&1; then break; fi
  sleep 0.5
done
echo "==> health: $(curl -fsS "http://$BIND/health" || echo 'NOT READY')"

# --- mcp proxy --------------------------------------------------------------
# Read-only MCP server in front of httpd. An AI assistant (Goose, Claude Code)
# connects here and drives the node read-only; it forwards each tool call to
# httpd over HTTP with the bearer token, never touching the seed.
if [ "${DEMO_NO_MCP:-0}" != "1" ]; then
  echo "==> starting landfall-mcp on http://$MCP_BIND/mcp — proxying to http://$BIND"
  "$MCP" \
    --bind "$MCP_BIND" \
    --base "http://$BIND" \
    --httpd-token "$TOKEN" &
  MCPPID=$!
fi

# --- web frontend -----------------------------------------------------------
if [ ! -f landfall-web/package.json ]; then
  echo "landfall-web/ not found — cannot start the UI." >&2
  exit 1
fi

URL="http://localhost:$WEB_PORT"
echo
echo "============================ DEMO RUNBOOK ============================"
echo "Open: $URL"
echo
if [ "${DEMO_REUSE:-0}" = "1" ]; then
  echo "  (reuse mode — wallet already provisioned)"
  echo "  1. Land on the DASHBOARD: live balances + node status (/node)."
  echo "  2. ACTIVITY: in/out, OCEAN payouts, LN vs on-chain (/activity)."
  echo "  3. RECEIVE: create a BOLT11 invoice (/invoice)."
  echo "  4. SEND: pay a BOLT11/BOLT12/LN-address — MOVES REAL FUNDS (/pay)."
else
  echo "  1. WELCOME -> generate a fresh 24-word seed (/generate)."
  echo "  2. Confirm the phrase; see the derived BIP84 mining address."
  echo "  3. WALLET: provision the Lexe node LIVE — real mainnet (/init).   <- the moment"
  echo "  4. Create the BOLT12 offer (/offer), then BIP-322 SIGN the OCEAN"
  echo "     message (/payout) — note the offer-in-message safety guard."
  echo "  5. DASHBOARD: live balances (/node), activity (/activity),"
  echo "     RECEIVE an invoice (/invoice), SEND a payment (/pay, real funds)."
fi
echo
if [ "${DEMO_NO_MCP:-0}" != "1" ]; then
  echo "MCP (read-only AI assistant) — already running, a nice closer if there's time:"
  echo "  Add to Goose / Claude Code:  http://$MCP_BIND/mcp  (no header needed)"
  echo "  e.g.  claude mcp add --transport http landfall http://$MCP_BIND/mcp"
  echo "  The dashboard's MCP panel shows the same command."
else
  echo "Tip: the dashboard's MCP panel shows the command to let an AI assistant"
  echo "     drive this same node read-only (DEMO_NO_MCP=1 skipped it here)."
fi
echo "Stop everything with Ctrl-C."
echo "====================================================================="
echo

if [ "${DEMO_NO_OPEN:-0}" != "1" ] && command -v open >/dev/null 2>&1; then
  ( sleep 2; open "$URL" ) &
fi

echo "==> starting landfall-web (Vite) on $URL — token injected"
( cd landfall-web
  [ -d node_modules ] || npm install
  VITE_LANDFALL_BASE="http://$BIND" VITE_LANDFALL_TOKEN="$TOKEN" \
    npm run dev -- --port "$WEB_PORT" --strictPort ) &
WEB=$!

wait "$WEB"
