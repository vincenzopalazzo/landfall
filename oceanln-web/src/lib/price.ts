// Live BTC/USD spot price for the dashboard's "Show USD" toggle.
//
// Source: mempool.space's public price endpoint (CORS-open, no API key,
// already a crypto-native dependency we link to for block/tx explorers).
// The value is cached in-module for 60s so flipping the sats/USD toggle
// doesn't refetch on every click. Returns 0 when the price is unavailable
// (offline, blocked, or a bad response) — callers treat 0 as "USD not
// available" and fall back to sats, never showing a fabricated fiat value.
//
// Desktop (Tauri) note: `https://mempool.space` is allow-listed in the
// app's CSP `connect-src` (see src-tauri/tauri.conf.json) so this fetch
// works in the packaged app, not just the browser build.

const PRICE_URL = "https://mempool.space/api/v1/prices";
const TTL_MS = 60_000;

let cached = 0;
let cachedAt = 0;

export async function btcUsd(): Promise<number> {
  const now = Date.now();
  if (cached && now - cachedAt < TTL_MS) return cached;
  try {
    const r = await fetch(PRICE_URL);
    if (!r.ok) return cached;
    const j = (await r.json()) as { USD?: number };
    const usd = Number(j?.USD);
    if (Number.isFinite(usd) && usd > 0) {
      cached = usd;
      cachedAt = now;
    }
  } catch {
    /* offline / blocked — keep the last good value (or 0) */
  }
  return cached;
}
