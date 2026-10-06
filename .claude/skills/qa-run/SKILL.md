---
name: qa-run
description: >-
  Check one change (a branch, a PR, a diff) against the landfall QA harness
  and report: plan what the change touches, run the scripted scenarios that
  apply, walk the matching registry sections, and post a PASS/FAIL table.
  Does not fix. Use when asked to "QA this PR", "verify this change", or
  before approving anything that touches signing, seeds, httpd or the wizard.
---

# landfall QA run (one change, report only)

```bash
scripts/qa/plan.sh --base origin/main          # RUN / NOT RUN / WALK for this diff
cargo build --release -p landfall-cli -p landfall-httpd --features landfall-httpd/qa-mock
# then exactly the RUN lines the planner printed, e.g.
scripts/qa/cli-smoke.sh
scripts/qa/httpd-smoke.sh
scripts/qa/web-e2e.sh
cargo test --all-targets --all-features
```

Rules:

- Run every RUN line; never skip one because it "cannot be affected".
- Each NOT RUN line goes into the report with its reason; do not claim
  coverage you did not execute.
- For each WALK section, re-read the scenarios against the diff: if the
  change alters an expectation, say so. A scenario marked **open** that the
  change fixes should be automated in the same PR.
- A `FAIL` is reported verbatim (the line plus the 20 log lines the runner
  prints). Do not re-run until green and report only the green run.
- Signing, seed or auth changes: name the exact tests that pin them
  (`sign.rs`, `bip322_vectors.rs`, `server.rs`) and whether they were
  extended.

Report format:

```
QA run — <branch/PR> at <sha>
RUN:      cli-smoke 10/10, httpd-smoke 24/24, web-e2e 2/2 (+QA-208, QA-209), cargo test ok
NOT RUN:  <script>: <reason>
WALK:     <section>: <ok | expectation changed: QA-xxx …>
FINDINGS: <none | QA-xxx FAIL … | new: …>
```
