// Read-only client for the public OCEAN pool API (https://api.ocean.xyz/v1).
// The pool serves `access-control-allow-origin: *`, so the browser fetches it
// directly — no proxy. Responses are wrapped as { "result": … } or { "error": … };
// numeric fields come back as strings (BTC amounts, hashes/sec, unix seconds).
//
// The OCEAN "username" is the miner's payout Bitcoin address — exactly the
// address this wallet derives, so the dashboard is keyed by address.

const BASE = "https://api.ocean.xyz/v1";

// Field names mirror the live API exactly (verified against a real address).
// Numbers arrive as strings; coerce with `num`/`btcToSats`/`hashesToThs`.
export interface StatSnap {
  snap_type: string; // "user"
  snap_ts: string;
  shares_60s: string;
  shares_300s: string;
  hashrate_60s: string;
  hashrate_300s: string;
  lastest_share_ts: string; // unix seconds (sic: OCEAN's spelling)
  shares_in_tides: string; // the user's share count in the current TIDES window
  estimated_earn_next_block: string; // BTC
  estimated_bonus_earn_next_block: string; // BTC
  estimated_total_earn_next_block: string; // BTC
  estimated_payout_next_block: string; // BTC
  unpaid: string; // BTC
}

// Per-address hashrate across several rolling windows (richer than statsnap's
// 60s/300s) plus the live worker count — the real source for "is this miner
// active and how fast".
export interface UserHashrate {
  snap_ts: string;
  db_ts: string;
  hashrate_60s: string;
  hashrate_300s: string;
  hashrate_600s: string;
  hashrate_1800s: string;
  hashrate_3600s: string;
  hashrate_10800s: string;
  hashrate_43200s: string;
  hashrate_86400s: string;
  active_worker_count: number;
  lastest_share_ts: string;
}

export interface Payout {
  ts: string | number;
  on_chain_txid: string;
  total_satoshis_net_paid: number | string;
  is_generation_txn: boolean;
}
export interface Earning {
  block_hash: string;
  ts: string | number;
  satoshis_net_earned: number | string;
}
export interface EarnPay {
  start_ts: string | number;
  end_ts: string | number;
  earnings: Earning[];
  payouts: Payout[];
}

export interface PoolStat {
  snap_ts: string;
  active_users: string;
  active_workers: string;
  network_difficulty: string;
  current_tides_shares: string;
  max_tides_shares: string;
  current_estimated_block_reward: string; // BTC
}

async function get<T>(path: string): Promise<T> {
  const resp = await fetch(`${BASE}${path}`);
  const body = await resp.json().catch(() => ({}));
  if (body && typeof body === "object" && "error" in body) {
    throw new Error(String((body as { error: unknown }).error));
  }
  if (!resp.ok) throw new Error(`OCEAN API ${resp.status}`);
  // Require the `{ result }` envelope — a 200 with any other shape (or a parse
  // fallback) must error, not hand back `undefined` for callers to deref.
  if (!body || typeof body !== "object" || !("result" in body)) {
    throw new Error(`OCEAN API: unexpected response for ${path}`);
  }
  return (body as { result: T }).result;
}

export const ocean = {
  statsnap: (address: string) => get<StatSnap>(`/statsnap/${encodeURIComponent(address)}`),
  earnpay: (address: string) => get<EarnPay>(`/earnpay/${encodeURIComponent(address)}`),
  userHashrate: (address: string) => get<UserHashrate>(`/user_hashrate/${encodeURIComponent(address)}`),
  poolStat: () => get<PoolStat>(`/pool_stat`),
};

// ── helpers ──
// OCEAN returns numbers as strings; coerce defensively so a missing/garbage
// field (null, "", "NaN") reads as 0 instead of poisoning the dashboard totals
// with NaN (which would render as "NaN sats").
export const num = (v: string | number | null | undefined): number => {
  const n = typeof v === "number" ? v : Number(v);
  return Number.isFinite(n) ? n : 0;
};
export const btcToSats = (btc: string | number | null | undefined): number =>
  Math.round(num(btc) * 1e8);
export const hashesToThs = (hps: string | number | null | undefined): number => num(hps) / 1e12;
