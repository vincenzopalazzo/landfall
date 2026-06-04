// Read-only client for the public OCEAN pool API (https://api.ocean.xyz/v1).
// The pool serves `access-control-allow-origin: *`, so the browser fetches it
// directly — no proxy. Responses are wrapped as { "result": … } or { "error": … };
// numeric fields come back as strings (BTC amounts, hashes/sec, unix seconds).
//
// The OCEAN "username" is the miner's payout Bitcoin address — exactly the
// address this wallet derives, so the dashboard is keyed by address.

const BASE = "https://api.ocean.xyz/v1";

export interface StatSnap {
  snap_ts: string;
  hashrate_60s: string;
  hashrate_300s: string;
  shares_in_tides: string;
  estimated_payout_next_block: string; // BTC
  unpaid: string; // BTC
  lastest_share_ts: string; // unix seconds (sic: OCEAN's spelling)
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
  active_users: string;
  active_workers: string;
  network_difficulty: string;
  current_estimated_block_reward: string;
}

async function get<T>(path: string): Promise<T> {
  const resp = await fetch(`${BASE}${path}`);
  const body = await resp.json().catch(() => ({}));
  if (body && typeof body === "object" && "error" in body) {
    throw new Error(String((body as { error: unknown }).error));
  }
  if (!resp.ok) throw new Error(`OCEAN API ${resp.status}`);
  return (body as { result: T }).result;
}

export const ocean = {
  statsnap: (address: string) => get<StatSnap>(`/statsnap/${encodeURIComponent(address)}`),
  earnpay: (address: string) => get<EarnPay>(`/earnpay/${encodeURIComponent(address)}`),
  poolStat: () => get<PoolStat>(`/pool_stat`),
};

// ── helpers ──
export const btcToSats = (btc: string | number): number => Math.round(Number(btc) * 1e8);
export const hashesToThs = (hps: string | number): number => Number(hps) / 1e12;
