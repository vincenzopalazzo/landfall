#!/usr/bin/env bash
# httpd-smoke.sh — registry scenarios QA-101…QA-121 against the compiled
# `landfall-httpd` binary over real HTTP (curl), offline. Needs the binary
# built with `--features qa-mock` for the wallet-touching scenarios
# (QA-111); without it those are SKIPped, everything else still runs.
#
#   scripts/qa/httpd-smoke.sh
#   LANDFALL_HTTPD=target/release/landfall-httpd LANDFALL=target/release/landfall scripts/qa/httpd-smoke.sh
#
# Two servers are started on ephemeral loopback ports: one in token mode,
# one in --no-auth mode. Both are killed on exit. Exit status = failures.
set -euo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

HTTPD="$(qa_resolve_bin LANDFALL_HTTPD landfall-httpd -p landfall-httpd --features landfall-httpd/qa-mock)"
BIN="$(qa_resolve_bin LANDFALL landfall -p landfall-cli)"
W="$QA_HOME/httpd"; rm -rf "$W"; mkdir -p "$W"
TOKEN="qa-httpd-token-$RANDOM"
ORIGIN="http://localhost:4173"
MOCK=()
if "$HTTPD" --help 2>/dev/null | grep -q -- '--mock-wallet'; then MOCK=(--mock-wallet); fi
echo "httpd-smoke: $HTTPD ${MOCK[*]:-(no qa-mock feature)}"

PIDS=()
cleanup() { for p in "${PIDS[@]:-}"; do [[ -n "$p" ]] && kill "$p" 2>/dev/null || true; done; }
trap cleanup EXIT INT TERM

start_httpd() { # <name> <seed-path> <extra args...> → prints base URL
  local name="$1" seed="$2"; shift 2
  local port; port="$(qa_free_port)"
  "$HTTPD" --bind "127.0.0.1:$port" --seed-file "$seed" --allow-origin "$ORIGIN" "${MOCK[@]}" "$@" >"$W/$name.log" 2>&1 &
  PIDS+=($!)
  qa_wait_http "http://127.0.0.1:$port/health" >&2
  echo "http://127.0.0.1:$port"
}

# curl helper: sets $CODE (status) and $BODY (response body) in THIS shell —
# no command substitution, or the variables would die with the subshell.
CODE=""; BODY=""
req() { # <method> <url> <json-body|""> [extra curl args...]
  local m="$1" u="$2" b="$3"; shift 3
  CODE="$(curl -sS -o "$W/body" -w '%{http_code}' -X "$m" "$u" -H "Content-Type: application/json" ${b:+--data "$b"} "$@")"
  BODY="$(cat "$W/body")"
}
auth=(-H "Authorization: Bearer $TOKEN")

# ════════════════════ token mode ════════════════════
BASE="$(start_httpd token "$W/seed" --token "$TOKEN")"

req GET "$BASE/health" ""; body="$BODY"; qa_check QA-101 "/health answers 200 without a token" test "$CODE" = 200
req GET "$BASE/status" ""; qa_check QA-102 "/status is 401 without a bearer token" test "$CODE" = 401
req GET "$BASE/status" "" "${auth[@]}"; body="$BODY"
qa_check QA-103 "/status with the token: configured=false before any seed exists" bash -c '[[ "$1" == 200 && "$(qa_json configured <<<"$2")" == False ]]' _ "$CODE" "$body"

req POST "$BASE/generate" "{}" "${auth[@]}"; body="$BODY"
PHRASE="$(qa_json mnemonic <<<"$body" 2>/dev/null || true)"
GEN_ADDR="$(qa_json mining_address <<<"$body" 2>/dev/null || true)"
qa_check QA-104 "/generate reveals 24 words + address once and writes the seed file 0600" bash -c '
  [[ "$1" == 200 && $(wc -w <<<"$2") -eq 24 && "$3" == bc1q* ]] && [[ "$(stat -c %a "$4" 2>/dev/null || stat -f %Lp "$4")" == 600 ]]' _ "$CODE" "$PHRASE" "$GEN_ADDR" "$W/seed"
req POST "$BASE/generate" "{}" "${auth[@]}" ; qa_check QA-105 "a second /generate is refused with 409 (never overwrite a wallet)" test "$CODE" = 409

req GET "$BASE/status" "" "${auth[@]}"; body="$BODY"
qa_check QA-106 "/status now reports configured=true with the generated address" bash -c '[[ "$(qa_json configured <<<"$1")" == True && "$(qa_json mining_address <<<"$1")" == "$2" ]]' _ "$body" "$GEN_ADDR"

hdrs="$(curl -sS -D - -o "$W/reveal.json" -X POST "$BASE/seed/reveal" "${auth[@]}" -H "Content-Type: application/json" --data '{}')"
qa_check QA-107 "/seed/reveal returns the stored phrase verbatim with Cache-Control: no-store" bash -c '
  grep -qi "^cache-control: no-store" <<<"$1" && [[ "$(qa_json mnemonic <"$2")" == "$3" ]]' _ "$hdrs" "$W/reveal.json" "$PHRASE"
req POST "$BASE/seed/reveal" "{}" ; qa_check QA-108 "/seed/reveal without a token is 401" test "$CODE" = 401

MSG="Configure OCEAN payout to $QA_MOCK_OFFER at block 840000"
req POST "$BASE/payout" "{\"message\":$(python3 -c 'import json,sys;print(json.dumps(sys.argv[1]))' "$MSG"),\"offer\":\"$QA_MOCK_OFFER\"}" "${auth[@]}"; body="$BODY"
SIG="$(qa_json signature <<<"$body" 2>/dev/null || true)"
qa_check QA-109 "/payout (offer supplied) signs offline for the configured address" bash -c '[[ "$1" == 200 && "$(qa_json address <<<"$2")" == "$3" && -n "$4" ]]' _ "$CODE" "$body" "$GEN_ADDR" "$SIG"
qa_check QA-110 "the CLI verifies the signature httpd produced (cross-transport BIP-322 check)" "$BIN" verify --address "$GEN_ADDR" --message "$MSG" --signature "$SIG"
req POST "$BASE/payout" '{"message":"Configure OCEAN payout to lno1other","offer":"'"$QA_MOCK_OFFER"'"}' "${auth[@]}"
qa_check QA-111 "/payout with an offer the message does not embed is 400" test "$CODE" = 400

if ((${#MOCK[@]})); then
  req POST "$BASE/init" "{}" "${auth[@]}"; body="$BODY"
  qa_check QA-112 "/init provisions (mock) and reports the address" bash -c '[[ "$1" == 200 && "$(qa_json provisioned <<<"$2")" == True ]]' _ "$CODE" "$body"
  req POST "$BASE/offer" '{"description":"QA"}' "${auth[@]}"; body="$BODY"
  qa_check QA-113 "/offer returns an lno1… offer (mock node)" bash -c '[[ "$1" == 200 && "$(qa_json offer <<<"$2")" == lno1* ]]' _ "$CODE" "$body"
  req GET "$BASE/node" "" "${auth[@]}"; body="$BODY"
  qa_check QA-114 "/node, /payouts, /activity answer 200 with the token" bash -c '
    [[ "$1" == 200 ]] && curl -fsS "$2/payouts" -H "Authorization: Bearer $3" >/dev/null && curl -fsS "$2/activity" -H "Authorization: Bearer $3" >/dev/null' _ "$CODE" "$BASE" "$TOKEN"
else
  qa_skip "QA-112 QA-113 QA-114 need landfall-httpd built with --features qa-mock"
fi

req GET "$BASE/status" "" "${auth[@]}" -H "Origin: https://evil.example" ; qa_check QA-115 "a request with a non-allowlisted Origin is 403 even with the token" test "$CODE" = 403
req GET "$BASE/status" "" "${auth[@]}" -H "Origin: $ORIGIN" ; qa_check QA-116 "the allow-listed Origin is accepted" test "$CODE" = 200
pre="$(curl -sS -D - -o /dev/null -X OPTIONS "$BASE/payout" -H "Origin: $ORIGIN" -H "Access-Control-Request-Method: POST" -H "Access-Control-Request-Headers: authorization,content-type")"
qa_check QA-117 "CORS preflight for the allow-listed Origin advertises POST + Authorization" bash -c 'grep -qi "access-control-allow-origin: $2" <<<"$1" && grep -qi "access-control-allow-headers:.*authorization" <<<"$1"' _ "$pre" "$ORIGIN"
req GET "$BASE/status" "" "${auth[@]}" -H "Host: wallet.example.com" ; qa_check QA-118 "a non-loopback Host header is 403 (DNS-rebinding guard)" test "$CODE" = 403

req POST "$BASE/import" "{\"mnemonic\":\"$QA_OTHER_MNEMONIC\"}" "${auth[@]}" ; qa_check QA-119 "/import over an existing different seed is 409 without force" test "$CODE" = 409
req POST "$BASE/import" "{\"mnemonic\":\"$QA_TEST_MNEMONIC\",\"force\":true}" "${auth[@]}"; body="$BODY"
qa_check QA-120 "/import with force replaces the wallet; /status shows the imported address" bash -c '
  [[ "$1" == 200 && "$(qa_json mining_address <<<"$2")" == "$3" ]] && [[ "$(curl -sS "$4/status" -H "Authorization: Bearer $5" | python3 -c "import sys,json;print(json.load(sys.stdin)[\"mining_address\"])")" == "$3" ]]' _ "$CODE" "$body" "$QA_TEST_ADDRESS" "$BASE" "$TOKEN"

# ════════════════════ --no-auth mode ════════════════════
echo "$QA_TEST_MNEMONIC" >"$W/na-seed"; chmod 600 "$W/na-seed"
NA="$(start_httpd no-auth "$W/na-seed" --no-auth)"
req GET "$NA/status" "" ; qa_check QA-121 "--no-auth: GET /status answers 200 without a token" test "$CODE" = 200
fails=""
for route in /generate /import /payout /offer /init /invoice /pay /seed/reveal; do
  case $route in
    /import) b="{\"mnemonic\":\"$QA_OTHER_MNEMONIC\",\"force\":true}" ;;
    /payout) b="{\"message\":\"Configure OCEAN payout to $QA_MOCK_OFFER\",\"offer\":\"$QA_MOCK_OFFER\"}" ;;
    /pay) b='{"payable":"lnbc1x"}' ;;
    *) b='{}' ;;
  esac
  req POST "$NA$route" "$b"
  [[ "$CODE" == 403 ]] || fails="$fails $route=$CODE"
done
qa_check QA-122 "--no-auth: every seed/key/state-touching POST is 403 (B2 regression)" test -z "$fails"
[[ -n "$fails" ]] && echo "      | not gated:$fails"
req GET "$NA/status" ""; body="$BODY"
qa_check QA-123 "--no-auth: the forced /import above did not replace the seed" bash -c '[[ "$(qa_json mining_address <<<"$1")" == "$2" ]]' _ "$body" "$QA_TEST_ADDRESS"

# ── bind guard ──
qa_check QA-124 "a routable --bind is refused at startup" bash -c '! "$1" --bind 0.0.0.0:0 --seed-file "$2" --no-auth >/dev/null 2>&1' _ "$HTTPD" "$W/na-seed"

qa_summary httpd-smoke
