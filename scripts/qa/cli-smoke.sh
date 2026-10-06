#!/usr/bin/env bash
# cli-smoke.sh — registry scenarios QA-001…QA-009 against the compiled
# `landfall` binary, fully offline (no Lexe, no sidecar, no network).
#
#   scripts/qa/cli-smoke.sh            # all CLI scenarios
#   LANDFALL=/path/to/landfall scripts/qa/cli-smoke.sh
#
# Exit status = number of failed scenarios. See docs/QA-SCENARIOS.md.
set -euo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

BIN="$(qa_resolve_bin LANDFALL landfall -p landfall-cli)"
W="$QA_HOME/cli"; rm -rf "$W"; mkdir -p "$W"
DEAD_URL="http://127.0.0.1:1"
echo "cli-smoke: $BIN"

# ── QA-001 generate: 24 words, unique, warnings off stdout ──
g1="$("$BIN" generate 2>"$W/g1.err")"; g2="$("$BIN" generate 2>/dev/null)"
qa_check QA-001 "generate prints 24 lowercase words on stdout, warning on stderr, unique per run" bash -c '
  [[ $(wc -w <<<"$1") -eq 24 ]] && [[ "$1" =~ ^[a-z\ ]+$ ]] && [[ "$1" != "$2" ]] && grep -q WARNING "$3" && ! grep -q WARNING <<<"$1"' _ "$g1" "$g2" "$W/g1.err"

# ── QA-002 generate --json ──
gj="$("$BIN" generate --json 2>/dev/null)"
qa_check QA-002 "generate --json is a {mnemonic} object with 24 words" bash -c '
  m=$(echo "$1" | python3 -c "import sys,json;print(json.load(sys.stdin)[\"mnemonic\"])"); [[ $(wc -w <<<"$m") -eq 24 ]]' _ "$gj"

# ── QA-003 init --dry-run derives the pinned address and writes nothing ──
if "$BIN" init --help >/dev/null 2>&1; then
  # An explicit --seed-file is read-first (it must exist), so isolate the
  # MANAGED seed dir instead and assert nothing lands there.
  out="$(echo "$QA_TEST_MNEMONIC" | XDG_CONFIG_HOME="$W/xdg" HOME="$W/home" "$BIN" init --dry-run --json 2>/dev/null || true)"
  qa_check QA-003 "init --dry-run derives $QA_TEST_ADDRESS and persists no seed" bash -c '
    [[ "$(echo "$1" | python3 -c "import sys,json;print(json.load(sys.stdin)[\"mining_address\"])")" == "$2" ]] && [[ ! -e "$3/landfall/seed" ]]' _ "$out" "$QA_TEST_ADDRESS" "$W/xdg"
else
  qa_skip "QA-003 init is not in this build (thin build)"
fi

# ── QA-004 payout --offer signs offline; verify accepts it ──
MSG="Configure OCEAN payout to $QA_MOCK_OFFER at block 840000"
po="$(echo "$QA_TEST_MNEMONIC" | "$BIN" payout --json --url "$DEAD_URL" --offer "$QA_MOCK_OFFER" --message "$MSG" 2>"$W/po.err" || true)"
ADDR="$(qa_json address <<<"$po" 2>/dev/null || true)"
SIG="$(qa_json signature <<<"$po" 2>/dev/null || true)"
qa_check QA-004 "payout --offer signs offline, echoes offer, address is the pinned one" bash -c '
  [[ "$1" == "$2" ]] && [[ -n "$3" ]] && [[ "$(echo "$4" | python3 -c "import sys,json;print(json.load(sys.stdin)[\"offer\"])")" == "$5" ]]' _ "$ADDR" "$QA_TEST_ADDRESS" "$SIG" "$po" "$QA_MOCK_OFFER"
qa_check QA-005 "verify accepts the signature payout produced (exit 0, valid:true)" bash -c '
  o=$("$1" verify --json --address "$2" --message "$3" --signature "$4") && [[ "$(echo "$o" | python3 -c "import sys,json;print(json.load(sys.stdin)[\"valid\"])")" == True ]]' _ "$BIN" "$ADDR" "$MSG" "$SIG"

# ── QA-006 verify rejects tampering / wrong address / garbage (exit 1, never a crash) ──
qa_check QA-006 "verify rejects a tampered message, another address, and garbage with exit 1" bash -c '
  set +e
  "$1" verify --address "$2" --message "$3 at block 840001" --signature "$4" >/dev/null 2>&1; a=$?
  "$1" verify --address bc1qcr8te4kr609gcawutmrza0j4xv80jy8z306fyu --message "$3" --signature "$4" >/dev/null 2>&1; b=$?
  "$1" verify --address "$2" --message "$3" --signature "nope" >/dev/null 2>&1; c=$?
  "$1" verify --address 1BitcoinEaterAddressDontSendf59kuE --message "$3" --signature "$4" >/dev/null 2>&1; d=$?
  [[ $a -eq 1 && $b -eq 1 && $c -eq 1 && $d -eq 1 ]]' _ "$BIN" "$ADDR" "$MSG" "$SIG"

# ── QA-007 the offer must be embedded in the message ──
qa_check QA-007 "payout refuses an --offer that the --message does not embed" bash -c '
  set +e; out=$(echo "$2" | "$1" payout --url http://127.0.0.1:1 --offer "$3" --message "Configure OCEAN payout to lno1other at block 1" 2>&1); rc=$?
  [[ $rc -eq 1 ]] && grep -q "not present in --message" <<<"$out"' _ "$BIN" "$QA_TEST_MNEMONIC" "$QA_MOCK_OFFER"

# ── QA-008 a different --path yields a different address, still self-consistent ──
po2="$(echo "$QA_TEST_MNEMONIC" | "$BIN" payout --json --url "$DEAD_URL" --offer "$QA_MOCK_OFFER" --message "$MSG" --path "m/84'/0'/0'/0/1" 2>/dev/null || true)"
qa_check QA-008 "payout --path m/84'/0'/0'/0/1 derives another address and its signature verifies" bash -c '
  a=$(echo "$2" | python3 -c "import sys,json;print(json.load(sys.stdin)[\"address\"])"); s=$(echo "$2" | python3 -c "import sys,json;print(json.load(sys.stdin)[\"signature\"])")
  [[ "$a" != "$3" ]] && "$1" verify --address "$a" --message "$4" --signature "$s" >/dev/null' _ "$BIN" "$po2" "$QA_TEST_ADDRESS" "$MSG"

# ── QA-009 a group/world-readable seed file is refused ──
echo "$QA_TEST_MNEMONIC" >"$W/loose"; chmod 644 "$W/loose"
qa_check QA-009 "a 0644 seed file is refused with a chmod 600 hint" bash -c '
  set +e; out=$("$1" payout --url http://127.0.0.1:1 --seed-file "$2" --offer "$3" --message "$4" 2>&1 </dev/null); rc=$?
  [[ $rc -eq 1 ]] && grep -qi "chmod 600" <<<"$out"' _ "$BIN" "$W/loose" "$QA_MOCK_OFFER" "$MSG"

# ── QA-010 clap usage errors exit 2 ──
qa_check QA-010 "--offer with --description is a usage error (exit 2); missing --message too" bash -c '
  set +e; "$1" payout --offer lno1x --description d --message m >/dev/null 2>&1; a=$?; "$1" payout >/dev/null 2>&1; b=$?; [[ $a -eq 2 && $b -eq 2 ]]' _ "$BIN"

# ── QA-011 a seed piped one word per line is accepted like a seed file ──
po3="$(printf '%s\n' $QA_TEST_MNEMONIC | "$BIN" payout --json --url "$DEAD_URL" --offer "$QA_MOCK_OFFER" --message "$MSG" 2>/dev/null || true)"
qa_check QA-011 "payout accepts the seed piped one word per line (multi-line stdin)" bash -c '
  [[ "$(echo "$1" | python3 -c "import sys,json;print(json.load(sys.stdin)[\"address\"])")" == "$2" ]]' _ "$po3" "$QA_TEST_ADDRESS"

# ── QA-012 a wallet stored before the rename to Landfall is still found ──
mkdir -p "$W/xdg-legacy/oceanln"
printf '%s\n' "$QA_TEST_MNEMONIC" >"$W/xdg-legacy/oceanln/seed"; chmod 600 "$W/xdg-legacy/oceanln/seed"
po4="$(XDG_CONFIG_HOME="$W/xdg-legacy" "$BIN" payout --json --url "$DEAD_URL" --offer "$QA_MOCK_OFFER" --message "$MSG" </dev/null 2>/dev/null || true)"
qa_check QA-012 "a seed only at the pre-rename oceanln/seed path is used, and no second seed is created" bash -c '
  [[ "$(echo "$1" | python3 -c "import sys,json;print(json.load(sys.stdin)[\"address\"])")" == "$2" ]] && [[ ! -e "$3/landfall/seed" ]]' _ "$po4" "$QA_TEST_ADDRESS" "$W/xdg-legacy"

qa_summary cli-smoke
