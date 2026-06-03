#!/usr/bin/env bash
#
# smoke.sh — end-to-end smoke test for the oceanln CLI.
#
# Resolves the oceanln binary, spins up a mock Lexe sidecar (inline python3
# HTTP server), and exercises the `generate` and `payout` subcommands,
# printing a PASS/FAIL line per check and a final summary.
#
# Requirements: bash, python3 (already used for the mock + JSON parsing).
#
set -euo pipefail

# ----------------------------------------------------------------------------
# Locate the project root (one level up from this script's scripts/ dir).
# ----------------------------------------------------------------------------
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
cd "${ROOT_DIR}"

PORT="${SMOKE_PORT:-5393}"
MOCK_URL="http://127.0.0.1:${PORT}"
DEAD_URL="http://127.0.0.1:1"
TEST_MNEMONIC="music mystery deliver gospel profit blanket leaf tell photo segment letter degree nice plastic duty canyon mammal marble bicycle economy unique find cream dune"

FAIL=0
PASS=0
MOCK_PID=""

# ----------------------------------------------------------------------------
# Cleanup: always kill the mock sidecar on exit.
# ----------------------------------------------------------------------------
cleanup() {
  if [[ -n "${MOCK_PID}" ]] && kill -0 "${MOCK_PID}" 2>/dev/null; then
    kill "${MOCK_PID}" 2>/dev/null || true
    wait "${MOCK_PID}" 2>/dev/null || true
  fi
}
trap cleanup EXIT INT TERM

pass() { PASS=$((PASS + 1)); echo "PASS: $*"; }
fail() { FAIL=$((FAIL + 1)); echo "FAIL: $*"; }

# ----------------------------------------------------------------------------
# 1. Resolve the binary.
# ----------------------------------------------------------------------------
BIN=""
if [[ -n "${OCEANLN:-}" ]]; then
  if [[ -x "${OCEANLN}" ]]; then
    BIN="${OCEANLN}"
    echo "binary: using \$OCEANLN -> ${BIN}"
  else
    echo "error: \$OCEANLN is set to '${OCEANLN}' but it is not an executable file" >&2
    exit 1
  fi
elif [[ -x "${ROOT_DIR}/target/release/oceanln" ]]; then
  BIN="${ROOT_DIR}/target/release/oceanln"
  echo "binary: using prebuilt ${BIN}"
else
  echo "binary: not found, building with 'cargo build --release'..."
  cargo build --release
  BIN="${ROOT_DIR}/target/release/oceanln"
  if [[ ! -x "${BIN}" ]]; then
    echo "error: build did not produce ${BIN}" >&2
    exit 1
  fi
  echo "binary: built ${BIN}"
fi

# ----------------------------------------------------------------------------
# 2. Start the mock sidecar in the background.
# ----------------------------------------------------------------------------
echo "mock: starting sidecar on ${MOCK_URL}"
python3 - "${PORT}" <<'PYEOF' &
import sys
from http.server import BaseHTTPRequestHandler, HTTPServer

PORT = int(sys.argv[1])
OFFER = "lno1qgsqvgnwgcg35z6ee2h3yczraddm72xrfua9uve2rlrm9deu7xyfzrcgqp0s"


class Handler(BaseHTTPRequestHandler):
    def do_POST(self):
        length = int(self.headers.get("Content-Length", 0))
        if length:
            self.rfile.read(length)
        if self.path == "/v2/node/create_offer":
            body = ('{"offer":"%s"}' % OFFER).encode()
            self.send_response(200)
            self.send_header("Content-Type", "application/json")
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)
        else:
            self.send_response(404)
            self.end_headers()

    def log_message(self, *args):  # silence default request logging
        pass


HTTPServer(("127.0.0.1", PORT), Handler).serve_forever()
PYEOF
MOCK_PID=$!

# ----------------------------------------------------------------------------
# Wait for the mock to be ready (retry-connect loop).
# ----------------------------------------------------------------------------
READY=0
for _ in $(seq 1 50); do
  if ! kill -0 "${MOCK_PID}" 2>/dev/null; then
    echo "error: mock sidecar process died during startup" >&2
    exit 1
  fi
  if python3 - "${PORT}" <<'PYEOF' 2>/dev/null
import socket, sys
s = socket.socket()
s.settimeout(0.5)
try:
    s.connect(("127.0.0.1", int(sys.argv[1])))
    sys.exit(0)
except OSError:
    sys.exit(1)
finally:
    s.close()
PYEOF
  then
    READY=1
    break
  fi
  sleep 0.1
done

if [[ "${READY}" -ne 1 ]]; then
  echo "error: mock sidecar did not become ready on ${MOCK_URL}" >&2
  exit 1
fi
echo "mock: ready (pid ${MOCK_PID})"
echo "----------------------------------------"

# ----------------------------------------------------------------------------
# 3. Checks.
# ----------------------------------------------------------------------------

# Check 1: `generate` outputs exactly 24 words (on stdout).
GEN_OUT="$("${BIN}" generate 2>/dev/null || true)"
WORDS="$(printf '%s' "${GEN_OUT}" | wc -w | tr -d '[:space:]')"
if [[ "${WORDS}" == "24" ]]; then
  pass "generate outputs exactly 24 words"
else
  fail "generate outputs ${WORDS} words (expected 24)"
fi

# Check 2: `generate --json` is valid JSON whose `mnemonic` has 24 words.
GEN_JSON="$("${BIN}" generate --json 2>/dev/null || true)"
if printf '%s' "${GEN_JSON}" | python3 -c '
import sys, json
data = json.load(sys.stdin)
m = data["mnemonic"]
assert isinstance(m, str), "mnemonic not a string"
n = len(m.split())
assert n == 24, f"mnemonic has {n} words"
' 2>/dev/null; then
  pass "generate --json is valid JSON with a 24-word mnemonic"
else
  fail "generate --json invalid or mnemonic does not have 24 words"
fi

# Use a freshly generated mnemonic if we got one, else the known test mnemonic.
if [[ "${WORDS}" == "24" ]]; then
  PAYOUT_MNEMONIC="${GEN_OUT}"
else
  PAYOUT_MNEMONIC="${TEST_MNEMONIC}"
fi

# Check 3: `payout --url <mock> --json` returns JSON with non-empty
# address (bc1q...), offer (lno1...), and signature.
PAYOUT_JSON="$(printf '%s\n' "${PAYOUT_MNEMONIC}" | \
  "${BIN}" payout --url "${MOCK_URL}" --message "hello ocean" --description "smoke offer" --json 2>/dev/null || true)"
if printf '%s' "${PAYOUT_JSON}" | python3 -c '
import sys, json
data = json.load(sys.stdin)
addr = data["address"]
offer = data["offer"]
sig = data["signature"]
assert isinstance(addr, str) and addr.startswith("bc1q"), f"bad address: {addr!r}"
assert isinstance(offer, str) and offer.startswith("lno1"), f"bad offer: {offer!r}"
assert isinstance(sig, str) and sig, f"bad signature: {sig!r}"
' 2>/dev/null; then
  pass "payout --json returns address (bc1q), offer (lno1), and signature"
else
  fail "payout --json missing/invalid address, offer, or signature"
fi

# Check 4: `payout` against a dead port exits non-zero and stderr mentions
# "could not reach sidecar".
set +e
PAYOUT_ERR="$(printf '%s\n' "${PAYOUT_MNEMONIC}" | \
  "${BIN}" payout --url "${DEAD_URL}" --message "hello ocean" --description "smoke offer" 2>&1 >/dev/null)"
DEAD_RC=$?
set -e
if [[ "${DEAD_RC}" -ne 0 ]] && printf '%s' "${PAYOUT_ERR}" | grep -q "could not reach sidecar"; then
  pass "payout against dead port exits non-zero with 'could not reach sidecar'"
else
  fail "payout against dead port: rc=${DEAD_RC}, stderr did not contain expected message"
fi

# Check 5: `payout` with no --message exits 2 (clap usage error).
set +e
printf '%s\n' "${PAYOUT_MNEMONIC}" | \
  "${BIN}" payout --url "${MOCK_URL}" --description "smoke offer" >/dev/null 2>&1
NOMSG_RC=$?
set -e
if [[ "${NOMSG_RC}" -eq 2 ]]; then
  pass "payout with no --message exits 2"
else
  fail "payout with no --message exited ${NOMSG_RC} (expected 2)"
fi

# ----------------------------------------------------------------------------
# 4. Summary.
# ----------------------------------------------------------------------------
echo "----------------------------------------"
echo "smoke: ${PASS} passed, ${FAIL} failed"
if [[ "${FAIL}" -ne 0 ]]; then
  exit 1
fi
exit 0
