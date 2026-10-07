#!/usr/bin/env bash
# plan.sh — read-only planner for a QA run: maps the change under test to the
# QA areas it touches, checks what this machine can run, and prints
#
#   RUN      the scripted scenarios to execute, as commands
#   NOT RUN  scripted scenarios this machine cannot run, with the reason
#   WALK     registry sections (docs/QA-SCENARIOS.md) to check by hand
#
#   scripts/qa/plan.sh                       # diff against origin/main
#   scripts/qa/plan.sh --base <ref> [--head <ref>]
#   scripts/qa/plan.sh --all                 # ignore the diff, plan everything
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/../.."
BASE=origin/main; HEAD=HEAD; ALL=0
while [[ $# -gt 0 ]]; do
  case "$1" in
    --base) BASE="$2"; shift 2 ;;
    --head) HEAD="$2"; shift 2 ;;
    --all) ALL=1; shift ;;
    -h|--help) sed -n '2,12p' "$0"; exit 0 ;;
    *) echo "unknown arg: $1" >&2; exit 2 ;;
  esac
done

if ((ALL)); then
  files="landfall-common/ landfall-cli/ landfall-httpd/ landfall-mcp/ landfall-web/ src-tauri/"
else
  git rev-parse --verify -q "$BASE" >/dev/null || { echo "no $BASE; use --base or --all" >&2; exit 2; }
  files="$(git diff --name-only "$BASE...$HEAD" 2>/dev/null || git diff --name-only "$BASE" "$HEAD")"
  echo "change: $BASE..$HEAD ($(wc -l <<<"$files" | tr -d ' ') files)"
fi

cli=0; httpd=0; web=0; mcp=0; tauri=0; docs=0
for f in $files; do
  case "$f" in
    landfall-common/*|Cargo.toml|Cargo.lock) cli=1; httpd=1; web=1; mcp=1 ;;
    landfall-cli/*) cli=1 ;;
    landfall-httpd/*) httpd=1; web=1; mcp=1 ;;
    landfall-web/*) web=1 ;;
    landfall-mcp/*) mcp=1 ;;
    src-tauri/*) tauri=1 ;;
    scripts/qa/*|docs/QA-SCENARIOS.md) cli=1; httpd=1; web=1 ;;
    README.md|docs/*) docs=1 ;;
  esac
done

have() { command -v "$1" >/dev/null 2>&1; }
chromium=""
if [[ -n "${LANDFALL_QA_CHROMIUM:-}" ]]; then chromium="$LANDFALL_QA_CHROMIUM"
elif [[ -n "${PLAYWRIGHT_BROWSERS_PATH:-}" && -x "$PLAYWRIGHT_BROWSERS_PATH/chromium" ]]; then chromium="$PLAYWRIGHT_BROWSERS_PATH/chromium"
elif [[ -d landfall-web/node_modules/@playwright && -d "${HOME}/.cache/ms-playwright" ]]; then chromium="(playwright cache)"; fi

echo
echo "RUN"
((cli)) && echo "  scripts/qa/cli-smoke.sh                      # QA-001…010 (offline CLI)"
((httpd)) && echo "  scripts/qa/httpd-smoke.sh                    # QA-101…124 (real HTTP, token + --no-auth)"
if ((web)); then
  if have node && [[ -n "$chromium" ]]; then
    echo "  scripts/qa/web-e2e.sh                        # QA-201…209 (headless wizard + landfall verify)"
  fi
fi
((httpd || cli)) && echo "  scripts/qa/flake-check.sh --runs 5           # interleaving flakes in the socket/seed-file suites"
echo "  cargo test --all-targets --all-features      # unit + integration suites"
((web)) && echo "  (cd landfall-web && npm run check && npm test)"

echo
echo "NOT RUN"
have cargo || echo "  everything Rust: cargo not on PATH"
have python3 || echo "  cli-smoke / httpd-smoke: python3 not on PATH (JSON helpers)"
if ((web)); then
  have node || echo "  web-e2e: node not on PATH"
  [[ -n "$chromium" ]] || echo "  web-e2e: no Chromium for Playwright — npx playwright install chromium, or set LANDFALL_QA_CHROMIUM"
fi
((mcp)) && echo "  landfall-mcp: no scripted scenario yet — QA-301 is manual (an MCP client against httpd)"
((tauri)) && echo "  src-tauri: no headless driver — QA-401 is manual on a desktop"

echo
echo "WALK (docs/QA-SCENARIOS.md)"
((cli)) && echo "  ## CLI"
((httpd)) && echo "  ## HTTP server"
((web)) && echo "  ## Web wizard (the BIP-322 UX)"
((mcp)) && echo "  ## MCP proxy"
((tauri)) && echo "  ## Desktop (Tauri)"
((docs)) && echo "  ## Docs: every command, flag and endpoint named in the diff exists (README vs --help / router)"
