# Agent QA harness

Scripts behind the `qa-run` skill (`.claude/skills/qa-run/SKILL.md`: check
one change, report) and the `qa-pass` skill (`.claude/skills/qa-pass/SKILL.md`:
find, fix, repeat). They let an agent, or a human, QA the whole
BIP-322 payout flow headlessly — CLI, loopback HTTP server, and the web wizard
in a real browser — with no Lexe node, no sidecar and no network. Every bug a
pass finds grows [`docs/QA-SCENARIOS.md`](../../docs/QA-SCENARIOS.md) and,
when it can be driven, one of the runners below.

```bash
scripts/qa/plan.sh                                   # what this change needs, what this machine can run
cargo build --release -p landfall-cli -p landfall-httpd --features landfall-httpd/qa-mock
scripts/qa/cli-smoke.sh                              # QA-001…  offline CLI, exit = failures
scripts/qa/httpd-smoke.sh                            # QA-101…  real HTTP, token + --no-auth modes
(cd landfall-web && npm ci && npx playwright install chromium)
scripts/qa/web-e2e.sh                                # QA-201…  headless wizard, then `landfall verify`
scripts/qa/flake-check.sh --runs 5                   # interleaving flakes in the socket/seed-file suites
```

| Script | Purpose |
|---|---|
| `plan.sh` | Read-only planner for `qa-run`: maps the diff (against `origin/main`, or `--base`/`--head`, or `--all`) to QA areas, checks the toolchain, prints RUN (commands), NOT RUN (with the reason) and WALK (registry sections). |
| `lib.sh` | Shared helpers: binary resolution (`$LANDFALL`, `$LANDFALL_HTTPD`, then `target/`, then build), `PASS`/`FAIL` lines with registry ids, ephemeral ports, JSON field extraction, the throwaway fixture phrase. |
| `cli-smoke.sh` | Registry scenarios QA-001…012 against the compiled `landfall`: generate, dry-run derivation, offline `payout --offer`, `verify` accept/reject matrix, offer-in-message gate, `--path`, seed-file permissions, usage errors. |
| `httpd-smoke.sh` | QA-101…124 against the compiled `landfall-httpd` over curl: auth, one-time `/generate`, 409 on overwrite, `/seed/reveal` no-store, offline `/payout` cross-checked by the CLI, Origin/Host/CORS guards, forced `/import`, the full `--no-auth` gating matrix, the loopback bind guard. Wallet routes (`/init`, `/offer`, `/node`) need the `qa-mock` build and are SKIPped otherwise. |
| `web-e2e.sh` | QA-201…213: builds a QA-only bundle (`dist-qa`, httpd base + token baked in), serves it with `vite preview`, starts a fresh `landfall-httpd --mock-wallet` per spec and runs the Playwright specs in `landfall-web/e2e/`. The create spec writes the address, message and signature the wizard showed; the script then runs `landfall verify` on them (QA-208) and re-derives the address from the revealed phrase (QA-209). |
| `flake-check.sh` | Reruns a crate's tests N times at default, 32 and 1 test threads; exit status = failed runs. |

Scratch state (seed files, logs, Playwright traces) lives in `.qa/` at the
repo root (gitignored); override with `QA_HOME`.

## Safety

- The fixture phrase in `lib.sh` is a throwaway shared by every test suite in
  the repo. **Never fund it.** The runners never read `~/.config/landfall/seed`:
  every seed file they touch is under `$QA_HOME` and created fresh.
- `--mock-wallet` exists only in a build with `--features qa-mock`; the
  release binary has no such flag. The mock never contacts Lexe and refuses
  to pay. Seed handling and BIP-322 signing are the real code paths.
- Nothing here reaches `api.ocean.xyz` or a Lexe node. The OCEAN dashboard,
  the Lexe provisioning path and real payouts are manual scenarios in the
  registry (QA-3xx…).

## Traps

- **Chromium.** Playwright 1.56 wants Chromium build 1194. On a machine with a
  pre-installed one, point `LANDFALL_QA_CHROMIUM` at the executable (the
  script picks up `$PLAYWRIGHT_BROWSERS_PATH/chromium` by itself); otherwise
  `npx playwright install chromium`.
- **Port 4173.** `vite preview` must sit on a fixed port because httpd's
  `--allow-origin` is matched exactly; set `LANDFALL_QA_WEB_PORT` if it is
  taken.
- **One httpd per spec.** The create and import specs each need an empty
  seed file; the script restarts httpd between them. Do not run two specs
  against one server.
- **Settings commit on blur.** Credentials typed in the Settings panel apply
  when the field loses focus or on Enter (QA-212), so a spec that types a
  token must blur the field before expecting a re-bootstrap.
- **`recover` pre-seeds.** `web-e2e.sh recover` writes the fixture phrase to
  the seed file (no `.offer`) before starting httpd; do not reuse that server
  for another spec.
