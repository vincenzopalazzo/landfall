// When does OCEAN's next Lightning payout run? OCEAN pays out 30 network
// blocks after it finds a block; finding another block before then restarts
// the count, and one payout then settles both. The math is pure; StatsGrid
// feeds it OCEAN's live pool numbers plus the chain tip from `tipHeight`.

/** Network blocks OCEAN waits after its latest block before paying out. */
export const PAYOUT_DELAY_BLOCKS = 30;
// mempool.space is already allow-listed in the Tauri CSP (see price.ts).
const TIP_URL = "https://mempool.space/api/blocks/tip/height";
// Past ~10 weeks of blocks the pool is too small (or so large the count never
// finishes) for an estimate to mean anything.
const MAX_BLOCKS = 10_000;

/** Fraction of network blocks OCEAN finds: its hashrate over the network's. */
export function poolBlockShare(poolHashrate: number, networkDifficulty: number): number {
  return poolHashrate / ((networkDifficulty * 2 ** 32) / 600);
}

/**
 * Network blocks until the next payout runs, as the median and the 90th
 * percentile. `sinceOceanBlock` counts the blocks mined after OCEAN's latest
 * one; at PAYOUT_DELAY_BLOCKS or more that payout already ran, and the next
 * one waits for a new OCEAN block. Null when no estimate makes sense.
 */
export function payoutEta(share: number, sinceOceanBlock: number): { median: number; p90: number } | null {
  if (!(share > 0 && share < 1) || !Number.isInteger(sinceOceanBlock) || sinceOceanBlock < 0) return null;
  // run[j]: chance the count stands at j; waiting: chance no payout is pending.
  const run = new Array<number>(PAYOUT_DELAY_BLOCKS).fill(0);
  let waiting = 0;
  if (sinceOceanBlock >= PAYOUT_DELAY_BLOCKS) waiting = 1;
  else run[sinceOceanBlock] = 1;
  let paid = 0;
  let median = 0;
  for (let n = 1; n <= MAX_BLOCKS; n++) {
    // Each block is OCEAN's with chance `share`, which (re)starts the count.
    const found = (waiting + run.reduce((a, b) => a + b, 0)) * share;
    paid += run[PAYOUT_DELAY_BLOCKS - 1] * (1 - share);
    for (let j = PAYOUT_DELAY_BLOCKS - 1; j > 0; j--) run[j] = run[j - 1] * (1 - share);
    run[0] = found;
    waiting *= 1 - share;
    if (!median && paid >= 0.5) median = n;
    if (paid >= 0.9) return { median, p90: n };
  }
  return null;
}

/** Current chain height. OCEAN's API doesn't serve it, so ask mempool.space. */
export async function tipHeight(): Promise<number> {
  const r = await fetch(TIP_URL);
  const height = Number(await r.text());
  if (!r.ok || !Number.isInteger(height) || height <= 0) throw new Error("mempool.space: no tip height");
  return height;
}
