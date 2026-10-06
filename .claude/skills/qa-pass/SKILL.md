---
name: qa-pass
description: >-
  Run an end-to-end QA pass of oceanln (CLI, loopback HTTP server, web wizard
  in a headless browser) with no Lexe node: find bugs in the BIP-322 payout
  flow, fix them with tests, and repeat on the fixed build until a round comes
  back clean. Every bug found grows docs/QA-SCENARIOS.md and, where it can be
  driven, scripts/qa or oceanln-web/e2e. Use when asked to "QA the app", "find
  bugs before a release", "test the wizard", or to re-verify a build.
---

# oceanln QA pass

A QA pass is a loop, not a checklist:

```
build → scripted smoke (cli, httpd, web) → scenario walk (registry)
      → exploratory pass → triage → fix + test (every transport)
      → grow the registry → commit → rebuild → repeat until a round finds nothing new
```

Read `scripts/qa/README.md` first: it lists the harness traps (Chromium
build, fixed preview port, one httpd per spec, Settings keystrokes).

## Ground rules

- **Throwaway seeds only.** The runners create every seed under `.qa/`;
  never point them at `~/.config/oceanln/seed`, never fund the fixture
  phrase, never run `scripts/lexe_e2e.sh` or `web-demo.sh` from a QA pass
  (they provision real mainnet wallets).
- **No secrets in output or commits.** Phrases and tokens from a run stay in
  `.qa/` (gitignored). A PASS/FAIL line never contains a phrase.
- **Every transport.** The same `oceanln-common` sits behind the CLI, HTTP,
  the wizard and Tauri. A bug on one is checked on the others; fix
  all in the same PR or record the gap in the PR body.
- **Signing and seed code needs a human review.** Report, do not merge.
- **A failing scenario is never a flake.** Rerun with `flake-check.sh` to
  prove interleaving, then fix the shared state; never skip or disable.

## 1. Build

```bash
cargo build --release -p oceanln-cli -p oceanln-httpd --features oceanln-httpd/qa-mock
(cd oceanln-web && npm ci)
scripts/qa/plan.sh --all          # or against the branch's diff
```

## 2. Scripted smoke

```bash
scripts/qa/cli-smoke.sh
scripts/qa/httpd-smoke.sh
scripts/qa/web-e2e.sh             # needs Chromium: OCEANLN_QA_CHROMIUM or `npx playwright install chromium`
```

Each prints one `PASS`/`FAIL` line per registry id and exits with the number
of failures. A `FAIL` is a bug or a regression: triage it before anything
else.

## 3. Scenario walk

Open `docs/QA-SCENARIOS.md`. For each section the change touches (the
planner's WALK list), run the manual scenarios and re-read the automated
ones against the build: do the expectations still describe what the code
does? Scenarios marked **open** are known bugs; check whether the build
fixes them and, if so, automate them and remove the mark.

## 4. Exploratory pass (the BIP-322 UX)

Walk the wizard as a miner who has never heard of BIP-322:

- Is every step's purpose clear without the Guided tooltips?
- Can you get stuck? Close the tab mid-flow, press Back everywhere, paste a
  wrong message, import over an existing wallet, change Settings mid-step.
- Does the signing step tell you *what* you signed and *with which* address?
- Copy each of the three values; feed them to `oceanln verify`.
- Repeat through the CLI (`generate` → `init --dry-run` → `payout --offer`
  → `verify`) and through curl against httpd. Any difference between the
  transports is a finding.

## 5. Triage and fix

For every finding:

1. Reproduce it with the smallest input; note the surface(s).
2. Write the failing test first — unit/integration at the call site, or a
   scripted scenario — then fix.
3. Add or extend the registry scenario (**Origin** = this pass).
4. Run `cargo fmt`, `cargo clippy --all-targets --all-features -D warnings`,
   `cargo test --all-targets --all-features`, the thin build, and
   `(cd oceanln-web && npm run check && npm test)`.
5. Commit with the scenario id in the message.

## 6. Repeat

Rebuild and rerun sections 2–4. The pass ends when one full round adds no
new finding. Report: the scenario ids run, the findings with their fixes,
the open scenarios left, and the exact commands a reviewer can rerun.
