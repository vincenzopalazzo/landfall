# Brainstorm: "when will my Lightning payout arrive?" on the dashboard

Prompted by a user report: *"How long does it take usually for a payout to
happen in lightning after configuring the payout? I used landfall to set up my
lightning."* Today the dashboard shows unpaid balance and next-block
estimates, and the empty state says only that a payout comes "once your unpaid
balance clears the minimum threshold". Nothing says **when**.

## How OCEAN times a Lightning payout

Stated by the maintainer. OCEAN's public docs do not describe it.

1. A miner earns only when **OCEAN finds a block** and their shares are in the
   TIDES window (8 × network difficulty worth of shares; `/docs/tides`).
2. The Lightning payout runs **30 network blocks after that OCEAN block**,
   plus the time OCEAN takes to execute the payouts.
3. If OCEAN finds **another block during those 30**, the counter **resets** to
   the new block. One payout then settles both.
4. A failed Lightning payout is retried on every block. Once the owed amount
   reaches 0.01048576 BTC it is paid on-chain instead (`/docs/lightning`).

One check backs this up. The payer note in our test fixture
(`landfall-common/src/lexe_wallet.rs:567`) names block 952554, and mempool.space
shows OCEAN mined that block. The note therefore names the OCEAN block whose
counter completed, not the block the payout ran in. That lets a user's own
payouts measure the real lag.

### What the numbers look like

A model over network blocks, with p = OCEAN's share of network blocks and
10-minute blocks, gives these waits. Today p is 4.2% from `/v1/pool_hashrate`
and 4.7% from the last 15 blocks in `/v1/blocks`.

| state when you look | median | 90% within | mean |
|---|---|---|---|
| counter idle (last payout already ran) | ~12.5 h | ~25 h | ~14 h |
| OCEAN just found a block (counter 0/30) | ~8.5 h | ~20 h | ~10.5 h |
| counter at 20/30 | ~1.7 h | ~13 h | ~5 h |

The naive "next OCEAN block" answer (~3.6 h average) is **about 3× too
optimistic**. The honest answer to the reporter is "usually half a day,
sometimes more than a day". The 30-block quiet run is what makes it slow, and
the resets are what make the tail long.

## Clarified Problem Statement

**Goal:** In the Profile's payout stats, show when the next OCEAN
Lightning payout is likely to run. Show it as a range, with a live 30-block
counter from OCEAN's last block, so a miner who just set up Landfall knows
what to expect.

**Decided (from the user):**
- Surface: `StatsGrid.svelte`. Only the Profile renders it (`compact`); the
  Lightning dashboard dropped it in 67edcba.
- Depth: **time only**. No expected-sats figure and no "is it working?"
  health check.
- Uncertainty: **range + last block**, e.g. "usually within ~9 h, 9 in 10
  within ~20 h · OCEAN's last block 1 h ago (#974003), payout counter 6/30".
- Model: the 30-quiet-block rule with resets, plus the payout execution time.

**Constraints:**
- No new backend dependency for the web build. The frontend already calls
  `api.ocean.xyz` from the browser (CORS-OK), and `mempool.space` is already
  allow-listed for Tauri (`price.ts`).
- `/v1/pool_hashrate` and `/v1/blocks` are **undocumented**. Parse them
  defensively, the same way `ocean.ts` treats every other OCEAN field. A missing
  field hides the card. It must not break the dashboard.
- The math is a pure function, unit-tested with vitest, with no `fetch` inside it.

**Non-goals:**
- An expected-amount or sats forecast. `Est. payout next block` already exists.
- Detecting broken setups (verification pending, no inbound liquidity, the
  Alby Hub BOLT12 issue).
- The CLI, MCP and Done screen. They can follow later and reuse the same model.
- Modelling a miner's TIDES ramp-up, or whether their share rounds to 0 sats.

**Success criteria:**
- With a live counter at k/30, the card shows the counter, OCEAN's last block
  (age and height), and a median plus 90th-percentile time to the next payout.
- With the counter idle (k ≥ 30), the card says the payout waits for OCEAN's
  next block, then 30 more, and shows the idle-state range.
- Vitest pins the model against the numbers in the table above, within a
  tolerance, for p ∈ {0.042, 0.047} and k ∈ {idle, 0, 20}.
- If any of the three inputs fails (pool hashrate or difficulty, last OCEAN
  block, chain tip), the card hides itself. The rest of StatsGrid renders
  unchanged.

## Approaches Considered

### Approach A: browser-side model in the web app (recommended)

- **Sketch:** Add `ocean.poolHashrate()` and `ocean.blocks()` to `ocean.ts`,
  and a tip-height fetch from `mempool.space/api/blocks/tip/height`. A new pure
  module `payoutEta.ts` runs a 31-state chain (waiting, or j quiet blocks) over
  about 3000 network blocks. It returns the CDF, the median and the 90th
  percentile. p comes from `pool_300s / (network_difficulty · 2³² / 600)`, and
  k = tip − last OCEAN height. StatsGrid renders a "Next Lightning payout" card
  and swaps the "No payouts yet" copy for the estimate.
- **Affected files:** `landfall-web/src/lib/ocean.ts`, new
  `landfall-web/src/lib/payoutEta.ts` (+ `.test.ts`),
  `landfall-web/src/lib/StatsGrid.svelte`, `ocean.test.ts`, and maybe the
  Tauri CSP if the tip-height path is not covered by the existing
  mempool.space entry.
- **Tradeoffs:** Fits the current posture, since the browser already talks to
  OCEAN. It is the smallest diff and gives fast feedback. The model is in
  TypeScript only, so CLI and MCP would need a port later. It depends on two
  undocumented endpoints.
- **Effort:** S–M.

### Approach B: Rust core estimator, served through httpd and Tauri IPC

- **Sketch:** Add `pool_hashrate` and `blocks` to `OceanClient`
  (`landfall-common/src/ocean.rs`), plus a `payout_eta` module with the same
  model, served as `GET /ocean/payout_eta` and as a Tauri command. The web
  layer only renders. CLI (`landfall payouts` empty state) and MCP get the
  same answer for free when they want it.
- **Affected files:** `landfall-common/src/ocean.rs`, new
  `landfall-common/src/payout_eta.rs`, `landfall-httpd/src/lib.rs`,
  `src-tauri/src/lib.rs`, `landfall-web/src/lib/api.ts` / `tauri.ts`,
  `StatsGrid.svelte`.
- **Tradeoffs:** One model and one set of tests, ready for CLI and MCP. But it
  adds a web → httpd → OCEAN path next to the direct browser fetches, and it
  needs a tip-height source on the server. That is more surface for a feature
  the user has scoped to the dashboard.
- **Effort:** M.

### Approach C: empirical replay of recent blocks, no probability model

- **Sketch:** Apply the 30-block rule to the heights in `/v1/blocks` to find
  when past payouts ran. Show "recent payouts ran every ~X h (a–b)" next to
  the live k/30 counter. The miner's own Lexe payouts
  (`client().payouts()`) give each block-to-payment lag. With enough of them,
  the median lag calibrates the "execution time" term.
- **Affected files:** `ocean.ts`, `StatsGrid.svelte`, a small
  `payoutReplay.ts`.
- **Tradeoffs:** No modelling assumptions, and the numbers are easy to explain.
  But `/v1/blocks` returns only 15 blocks (~2 days, so a handful of payout
  events), which is far too noisy for a 90% bound. It also cannot say anything
  for a fresh miner with no payouts.
- **Effort:** S.

## Recommendation

**A**, using one idea from **C**: show the live k/30 counter, OCEAN's last
block, and the model's median and 90% figures. A is the smallest change that
answers the reporter's question with the correct 30-block rule, and it matches
how the dashboard already loads OCEAN data. Keep `payoutEta.ts` pure and
dependency-free, so moving it into `landfall-common` (B) is a port and not a
redesign once CLI or MCP need it.

## Open questions

- **Execution time:** how long after the 30th block do payouts land? It is a
  constant until someone measures it (payment `created_at` minus the payer-note
  block's time, from the miner's own Lexe history).
- **First payout after setup:** if a counter is already running when the
  config activates (block height "latest"), does that payout use Lightning
  for this miner? Or does the first one need an OCEAN block found after
  activation? This changes the copy for brand-new users.
- **Exact counter rule:** "30 blocks after" means the payout runs at height
  H_ocean + 30. Confirm whether it is +30 or +31, and whether the reset
  counts from the new block's height.
- **Endpoint stability:** are `/v1/pool_hashrate` and `/v1/blocks` meant to be
  public? If not, the tip and last-block height could come from mempool.space
  alone (pool attribution via `extras.pool`).
- **Signature verification:** is it instant? If not, the card should say the
  counter only applies once ocean.xyz shows Lightning as enabled.
