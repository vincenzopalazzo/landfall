import { render, screen } from "@testing-library/svelte";
import { describe, it, expect, beforeEach, vi } from "vitest";
import Dashboard from "./Dashboard.svelte";
import * as S from "./store.svelte";

function res(body: unknown): Response {
  return new Response(JSON.stringify(body), { status: 200, headers: { "content-type": "application/json" } });
}

function routeOcean() {
  globalThis.fetch = vi.fn(async (url: string | URL | Request) => {
    const u = String(url);
    if (u.includes("/statsnap/"))
      return res({ result: { snap_ts: "1700000000", hashrate_60s: "0", hashrate_300s: "2000000000000", shares_in_tides: "262144", estimated_payout_next_block: "0.0002", estimated_earn_next_block: "0.00015", unpaid: "0.001", lastest_share_ts: "1700000000" } });
    if (u.includes("/user_hashrate/"))
      return res({ result: { snap_ts: "1700000000", db_ts: "1700000000", hashrate_60s: "0", hashrate_300s: "2000000000000", hashrate_600s: "0", hashrate_1800s: "0", hashrate_3600s: "1500000000000", hashrate_10800s: "1800000000000", hashrate_43200s: "0", hashrate_86400s: "0", active_worker_count: 3, lastest_share_ts: "1700000000" } });
    if (u.includes("/earnpay/"))
      return res({ result: { start_ts: 0, end_ts: 0, earnings: [{ block_hash: "00000000000000000001abc", ts: "2026-06-07T03:35:48", satoshis_net_earned: 4 }, { block_hash: "00000000000000000001def", ts: "2026-06-06T01:08:34", satoshis_net_earned: 1 }], payouts: [{ ts: "1700000000", on_chain_txid: "abcdef1234567890", total_satoshis_net_paid: 12345, is_generation_txn: false }, { ts: "1699000000", on_chain_txid: "fffefefefefefefe", total_satoshis_net_paid: 333000000, is_generation_txn: true }] } });
    if (u.includes("/pool_stat")) return res({ result: { active_users: "2410", active_workers: "86080", network_difficulty: "1", current_tides_shares: "26214400", max_tides_shares: "26214400", current_estimated_block_reward: "3.2" } });
    return new Response("not found", { status: 404 });
  }) as typeof fetch;
}

beforeEach(() => {
  S.restart();
  S.app.miningAddress = "bc1qpstw48j7j9gjugw25jmjvd96jlwgdnedk5pr6r";
  S.app.offer = "lno1testoffer";
  routeOcean();
});

describe("Dashboard (live OCEAN data)", () => {
  it("renders real stats and the payouts table from the OCEAN API", async () => {
    render(Dashboard);
    // hashrate 2e12 h/s → 2.00 Th/s, so the miner shows as Mining
    expect(await screen.findByText("Mining")).toBeInTheDocument();
    // unpaid 0.001 BTC = 100,000 sats
    expect(await screen.findByText("100,000")).toBeInTheDocument();
    // payout row: 12,345 sats (regular) — coinbase row also appears
    expect(await screen.findByText(/12,345 sats/)).toBeInTheDocument();
    expect(screen.getByText("2.00")).toBeInTheDocument(); // Th/s (5m, from statsnap)
    // user_hashrate wiring: live worker count + 1h average rendered.
    expect(screen.getByText("Workers")).toBeInTheDocument();
    expect(screen.getByText("3")).toBeInTheDocument(); // active_worker_count
    expect(screen.getByText(/1\.50 Th\/s/)).toBeInTheDocument(); // hashrate_3600s
    // 3h average from hashrate_10800s = 1.8e12 → 1.80 Th/s
    expect(screen.getByText(/1\.80 Th\/s/)).toBeInTheDocument();
    // Recent earnings panel now renders the per-block earnings from earnpay.
    expect(await screen.findByText("Recent earnings")).toBeInTheDocument();
    expect(screen.getByText(/4 sats/)).toBeInTheDocument(); // first earning
    expect(screen.getByText(/1 sats/)).toBeInTheDocument(); // second earning
  });

  it("renders the new lifetime / est-earn / blocks / share% fields", async () => {
    // Total paid (12,345) + coinbase (333,000,000) = 333,012,345 paid;
    // lifetime = 333,012,345 + 100,000 unpaid = 333,112,345.
    // Blocks found = 1 (the coinbase row). Est. earn next block = 15,000 sats.
    // Share % = 262144 / 26214400 = 1.00%.
    //
    // Each value lands after the async load() resolves, so use
    // findByText (which retries) instead of getByText for the first
    // assertion in each derived chain.
    render(Dashboard);
    expect(await screen.findByText("333,112,345")).toBeInTheDocument(); // lifetime sats
    expect(screen.getByText("Lifetime")).toBeInTheDocument();
    expect(await screen.findByText("15,000")).toBeInTheDocument(); // est_earn = 0.00015 BTC
    expect(screen.getByText("Est. earn next block")).toBeInTheDocument();
    expect(screen.getByText("Blocks found")).toBeInTheDocument();
    expect(screen.getByText("Share %")).toBeInTheDocument();
    // Share % depends on pool_stat which loads AFTER the main fetch
    // resolves — so the value lands a tick later than the other cards.
    // findByText polls until it appears (or times out).
    expect(await screen.findByText("1.00%")).toBeInTheDocument();
  });

  it("headlines a longer window when the 5m hashrate is zero (intermittent miner)", async () => {
    // Real case: idle the last 5m (hashrate_300s=0) but mined this hour. The
    // headline must show the 1h hashrate, not a misleading "0 h/s".
    globalThis.fetch = vi.fn(async (url: string | URL | Request) => {
      const u = String(url);
      if (u.includes("/statsnap/"))
        return res({ result: { snap_ts: "1700000000", hashrate_60s: "0", hashrate_300s: "0", shares_in_tides: "262144", estimated_payout_next_block: "0", unpaid: "0", lastest_share_ts: "1700000000" } });
      if (u.includes("/user_hashrate/"))
        return res({ result: { snap_ts: "1", db_ts: "1", hashrate_60s: "0", hashrate_300s: "0", hashrate_600s: "0", hashrate_1800s: "0", hashrate_3600s: "625500000000", hashrate_10800s: "0", hashrate_43200s: "0", hashrate_86400s: "0", active_worker_count: 0, lastest_share_ts: "1700000000" } });
      if (u.includes("/earnpay/")) return res({ result: { earnings: [], payouts: [] } });
      if (u.includes("/pool_stat")) return res({ result: { active_users: "1" } });
      return new Response("not found", { status: 404 });
    }) as typeof fetch;
    render(Dashboard);
    expect(await screen.findByText("Hashrate (1h)")).toBeInTheDocument(); // adaptive label
    expect(screen.getByText("625.50")).toBeInTheDocument(); // 1h value, not 0
    expect(screen.queryByText("Hashrate (5m)")).toBeNull();
  });

  it("flags a payout-history failure instead of silently showing 'no payouts'", async () => {
    // statsnap succeeds but earnpay genuinely fails: the payouts table must say
    // it couldn't load, not imply an accurate empty history.
    globalThis.fetch = vi.fn(async (url: string | URL | Request) => {
      const u = String(url);
      if (u.includes("/statsnap/"))
        return res({ result: { snap_ts: "1700000000", hashrate_60s: "0", hashrate_300s: "2000000000000", shares_in_tides: "0", estimated_payout_next_block: "0.0002", unpaid: "0.001", lastest_share_ts: "1700000000" } });
      if (u.includes("/earnpay/")) return res({ error: "internal server error" });
      if (u.includes("/pool_stat")) return res({ result: { active_users: "1" } });
      return new Response("not found", { status: 404 });
    }) as typeof fetch;
    render(Dashboard);
    expect(await screen.findByText(/Couldn't load payout history/i)).toBeInTheDocument();
    expect(screen.queryByText(/No payouts yet/i)).toBeNull();
    // statsnap was fine: "100,000" appears for both Unpaid and Lifetime
    // (totalPaid=0 when earnpay 500s), so just assert it's present somewhere.
    expect(screen.getAllByText("100,000").length).toBeGreaterThanOrEqual(1);
  });

  it("shows an empty state for an address with no OCEAN history", async () => {
    globalThis.fetch = vi.fn(async (url: string | URL | Request) => {
      const u = String(url);
      if (u.includes("/statsnap/")) return res({ error: "No such user or user has no active workers" });
      if (u.includes("/earnpay/")) return res({ result: { earnings: [], payouts: [] } });
      return res({ result: { active_users: "1" } });
    }) as typeof fetch;
    render(Dashboard);
    expect(await screen.findByText(/No payouts yet/i)).toBeInTheDocument();
  });
});
