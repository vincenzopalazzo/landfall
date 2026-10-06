#!/usr/bin/env bash
# lib.sh — shared helpers for the scripts/qa/*.sh runners. Source it.
#
#   QA_HOME     scratch dir (seed files, logs, artefacts). Default: <repo>/.qa
#   OCEANLN     path to the `oceanln` binary (default: target/release, else build)
#   OCEANLN_HTTPD  path to `oceanln-httpd` (same resolution; QA needs the
#               `qa-mock` feature for --mock-wallet)
#
# Every scenario reports one line, `PASS QA-NNN …` or `FAIL QA-NNN …`, so a
# run's output can be pasted into a PR as-is; the exit status is the number
# of failed scenarios. Scenario ids are the registry's (docs/QA-SCENARIOS.md).

QA_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
QA_HOME="${QA_HOME:-$QA_ROOT/.qa}"
mkdir -p "$QA_HOME"

QA_PASS=0
QA_FAIL=0
QA_SKIP=0
QA_FAILED_IDS=()

qa_pass() { QA_PASS=$((QA_PASS + 1)); echo "PASS $*"; }
qa_fail() { QA_FAIL=$((QA_FAIL + 1)); QA_FAILED_IDS+=("$1"); echo "FAIL $*"; }
qa_skip() { QA_SKIP=$((QA_SKIP + 1)); echo "SKIP $*"; }

# qa_check <id> <description> <command...>: PASS when the command exits 0.
qa_check() {
  local id="$1" desc="$2"; shift 2
  if "$@" >"$QA_HOME/last.out" 2>&1; then
    qa_pass "$id $desc"
  else
    qa_fail "$id $desc"
    sed 's/^/      | /' "$QA_HOME/last.out" | head -20
  fi
}

qa_summary() {
  local name="$1"
  echo
  echo "$name: $QA_PASS passed, $QA_FAIL failed, $QA_SKIP skipped"
  if ((QA_FAIL > 0)); then
    echo "failed: ${QA_FAILED_IDS[*]}"
  fi
  exit $((QA_FAIL > 255 ? 255 : QA_FAIL))
}

# Resolve a workspace binary: $VAR, then target/release, then target/debug,
# else build it with the given cargo args.
qa_resolve_bin() {
  local var="$1" name="$2"; shift 2
  local v="${!var:-}"
  if [[ -n "$v" ]]; then
    [[ -x "$v" ]] || { echo "error: \$$var=$v is not executable" >&2; exit 2; }
    echo "$v"; return
  fi
  for d in release debug; do
    if [[ -x "$QA_ROOT/target/$d/$name" ]]; then echo "$QA_ROOT/target/$d/$name"; return; fi
  done
  echo "building $name ($*)…" >&2
  (cd "$QA_ROOT" && cargo build --release "$@" >&2)
  echo "$QA_ROOT/target/release/$name"
}

qa_free_port() {
  python3 -c 'import socket;s=socket.socket();s.bind(("127.0.0.1",0));print(s.getsockname()[1])'
}

# qa_wait_http <url> [seconds]
qa_wait_http() {
  local url="$1" n="${2:-30}"
  for ((i = 0; i < n * 10; i++)); do
    if curl -fsS -o /dev/null "$url" 2>/dev/null; then return 0; fi
    sleep 0.1
  done
  echo "timeout waiting for $url" >&2
  return 1
}

# JSON field extraction without jq: qa_json <field> <<<"$json"
qa_json() { python3 -c 'import sys,json;d=json.load(sys.stdin);v=d
for k in sys.argv[1].split("."):
    v=v[int(k)] if isinstance(v,list) else v[k]
print(v if not isinstance(v,(dict,list)) else json.dumps(v))' "$1"; }
# The runners call it from `bash -c` children, so it must be exported.
export -f qa_json

# The throwaway fixture phrase used across the repo's test suites. NEVER fund it.
QA_TEST_MNEMONIC="music mystery deliver gospel profit blanket leaf tell photo segment letter degree nice plastic duty canyon mammal marble bicycle economy unique find cream dune"
QA_TEST_ADDRESS="bc1qpstw48j7j9gjugw25jmjvd96jlwgdnedk5pr6r"
QA_OTHER_MNEMONIC="abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon art"
QA_MOCK_OFFER="lno1qgsqvgnwgcg35z6ee2h3yczraddm72xrfua9uve2rlrm9deu7xyfzrcgqp0s"
