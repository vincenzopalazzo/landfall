#!/usr/bin/env bash
# End-to-end demo of the OCEAN Lightning core, fully offline (no Lexe node).
#
# Drives the real landfall-httpd server over loopback to show the production code
# path the web wizard and the Tauri desktop shell both use: a fresh wallet, the
# derived BIP84 mining address, a real BIP-322 signature, and the safety guards.
# The desktop IPC commands wrap the same `landfall_httpd::service` functions, so
# this exercises that backend logic too.
#
# What needs a real Lexe node (NOT covered here): /offer and /init (provisioning
# + payable BOLT12 offer creation). Everything below is offline crypto.
#
# Usage: scripts/demo.sh      (builds the release server if needed)
set -euo pipefail
cd "$(dirname "$0")/.."

HTTPD=target/release/landfall-httpd
[ -x "$HTTPD" ] || { echo "==> building $HTTPD"; cargo build --release -p landfall-httpd; }

DEMO="$(mktemp -d)/landfall-demo"; mkdir -p "$DEMO"
TOKEN="demo-token-$$"
PORT="${LANDFALL_DEMO_PORT:-7801}"
CT="Content-Type: application/json"
BASE="http://127.0.0.1:$PORT"

echo "==> starting the server (loopback only, bearer-token guarded), no seed yet"
"$HTTPD" --bind "127.0.0.1:$PORT" --seed-file "$DEMO/seed" --token "$TOKEN" >"$DEMO/server.log" 2>&1 &
SRV=$!
trap 'kill $SRV 2>/dev/null || true; rm -rf "$DEMO"' EXIT
sleep 1

echo "==> 1. health (no auth required)"; curl -fsS "$BASE/health"; echo
echo "==> 2. guard: /generate without a token"
echo "    HTTP $(curl -s -o /dev/null -w '%{http_code}' -X POST "$BASE/generate")  (expect 401)"

echo "==> 3. generate a real wallet (seed + BIP84 mining address; no Lexe)"
GEN=$(curl -fsS -X POST "$BASE/generate" -H "Authorization: Bearer $TOKEN")
ADDR=$(echo "$GEN" | python3 -c 'import sys,json;print(json.load(sys.stdin)["mining_address"])')
echo "    mining address: $ADDR"
echo "    seed perms: $(stat -f '%Sp' "$DEMO/seed" 2>/dev/null || stat -c '%A' "$DEMO/seed")  (expect -rw-------)"

echo "==> 4. offline BIP-322 sign of an OCEAN-style message embedding the offer"
OFFER="lno1demoplaceholderoffer"
MSG="Configure OCEAN payout to $ADDR via $OFFER at block 900000"
curl -fsS -X POST "$BASE/payout" -H "Authorization: Bearer $TOKEN" -H "$CT" \
  -d "{\"message\":\"$MSG\",\"offer\":\"$OFFER\"}" \
  | python3 -c 'import sys,json;d=json.load(sys.stdin);print("    address:",d["address"]);print("    sig    :",d["signature"][:64],"... (",len(d["signature"]),"chars )")'

echo "==> 5. guard: a message that does NOT contain the offer is rejected"
bad_code=$(curl -s -o /dev/null -w '%{http_code}' -X POST "$BASE/payout" \
  -H "Authorization: Bearer $TOKEN" -H "$CT" \
  -d "{\"message\":\"no offer\",\"offer\":\"$OFFER\"}")
echo "    HTTP $bad_code  (expect 400 — offer-in-message guard)"

echo "==> 6. guard: re-generating over an existing wallet is refused (no fund-loss overwrite)"
clobber_code=$(curl -s -o /dev/null -w '%{http_code}' -X POST "$BASE/generate" -H "Authorization: Bearer $TOKEN")
echo "    HTTP $clobber_code  (expect 409 — SeedExists)"

echo "==> demo complete."
