#!/usr/bin/env bash
# End-to-end Lexe smoke: create a wallet -> create an offer with a custom
# description -> sign an OCEAN-style JSON config message.
#
# This drives the REAL in-process Lexe SDK path (no sidecar), so it requires:
#   - oceanln built with `--features lexe-sdk`
#   - network access to Lexe's MAINNET backend
#   - python3
#
# WARNING: `oceanln init` provisions a REAL mainnet Lexe wallet for the seed.
# By default this script uses a persistent throwaway seed at
# $HOME/.oceanln-smoke-seed (created once, reused on re-runs so it doesn't spam
# Lexe with new accounts). Override with SEED_FILE=/path to use your own seed.
#
# This is NOT a CI test — it is a manual end-to-end check. The CI-safe portion
# (signing the OCEAN JSON message) is covered by `payout_signs_ocean_json_message`
# in tests/smoke.rs.
#
# Env knobs: OCEANLN, SEED_FILE, DESCRIPTION, HEIGHT.
set -euo pipefail

OCEANLN="${OCEANLN:-$(command -v oceanln || echo target/release/oceanln)}"
SEED_FILE="${SEED_FILE:-$HOME/.oceanln-smoke-seed}"
DESCRIPTION="${DESCRIPTION:-OCEAN payout (oceanln e2e)}"
HEIGHT="${HEIGHT:-944040}"

fail() { echo "FAIL: $*" >&2; exit 1; }

command -v python3 >/dev/null 2>&1 || fail "python3 is required"
[ -x "$OCEANLN" ] || command -v "$OCEANLN" >/dev/null 2>&1 || fail "oceanln not found ($OCEANLN)"

# The in-process path only exists when built with --features lexe-sdk.
if ! "$OCEANLN" init --help >/dev/null 2>&1; then
  fail "this oceanln has no 'init' command — rebuild with: cargo install --path . --features lexe-sdk"
fi

echo "binary:       $OCEANLN"
echo "seed file:    $SEED_FILE"
echo "description:  $DESCRIPTION"
echo "height:       $HEIGHT"
echo "WARNING: 'init' provisions a REAL mainnet Lexe wallet for this seed."
echo "----------------------------------------"

# 1. Seed (persistent so re-runs reuse the same test wallet)
if [ ! -f "$SEED_FILE" ]; then
  ( umask 077; "$OCEANLN" generate 2>/dev/null > "$SEED_FILE" )
  chmod 600 "$SEED_FILE"
  echo "step 1: generated a fresh 24-word seed at $SEED_FILE"
else
  echo "step 1: reusing existing seed at $SEED_FILE"
fi
WORDS=$(wc -w < "$SEED_FILE" | tr -d ' ')
[ "$WORDS" = "24" ] || fail "seed file does not contain 24 words (got $WORDS)"

# 2. Create + provision the onchain wallet (idempotent)
echo "step 2: creating + provisioning the onchain Lexe wallet..."
"$OCEANLN" init < "$SEED_FILE" || fail "init failed"

# 3. Create the offer with a custom description
echo "step 3: creating a BOLT12 offer (description: '$DESCRIPTION')..."
OFFER=$("$OCEANLN" offer --json --description "$DESCRIPTION" < "$SEED_FILE" \
  | python3 -c 'import sys,json; print(json.load(sys.stdin)["offer"])') \
  || fail "offer creation failed"
case "$OFFER" in
  lno1*) echo "        offer: $OFFER" ;;
  *) fail "offer did not start with lno1: $OFFER" ;;
esac

# 4. Build the OCEAN-style JSON message embedding the offer, and sign it
MESSAGE=$(python3 -c \
  'import json,sys; print(json.dumps({"height": int(sys.argv[1]), "lightning_bolt12": sys.argv[2]}, separators=(",", ":")))' \
  "$HEIGHT" "$OFFER")
echo "step 4: signing OCEAN JSON message:"
echo "        $MESSAGE"
OUT=$("$OCEANLN" payout --json --offer "$OFFER" --message "$MESSAGE" < "$SEED_FILE") \
  || fail "payout (sign) failed"

# 5. Verify the signed result ($OUT on stdin, message as argv[1])
echo "$OUT" | python3 -c '
import json, sys
want_msg = sys.argv[1]
d = json.load(sys.stdin)
assert d["message"] == want_msg, "message not signed verbatim"
assert d["address"].startswith("bc1q"), f"bad address: {d[\"address\"]}"
assert d["offer"].startswith("lno1"), f"bad offer: {d[\"offer\"]}"
assert d["signature"], "empty signature"
print("        address:   " + d["address"])
print("        signature: " + d["signature"][:24] + "... (" + str(len(d["signature"])) + " chars)")
' "$MESSAGE" || fail "verification failed"

echo "----------------------------------------"
echo "PASS: wallet created -> offer created -> OCEAN JSON message signed"
