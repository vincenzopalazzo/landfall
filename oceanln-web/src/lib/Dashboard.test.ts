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
      return res({ result: { snap_ts: "1700000000", hashrate_60s: "0", hashrate_300s: "2000000000000", shares_in_tides: "262144", estimated_payout_next_block: "0.0002", unpaid: "0.001", lastest_share_ts: "1700000000" } });
    if (u.includes("/user_hashrate/"))
      return res({ result: { snap_ts: "1700000000", db_ts: "1700000000", hashrate_60s: "0", hashrate_300s: "2000000000000", hashrate_600s: "0", hashrate_1800s: "0", hashrate_3600s: "1500000000000", hashrate_10800s: "0", hashrate_43200s: "0", hashrate_86400s: "0", active_worker_count: 3, lastest_share_ts: "1700000000" } });
    if (u.includes("/earnpay/"))
      return res({ result: { start_ts: 0, end_ts: 0, earnings: [], payouts: [{ ts: "1700000000", on_chain_txid: "abcdef1234567890", total_satoshis_net_paid: 12345, is_generation_txn: false }] } });
    if (u.includes("/pool_stat")) return res({ result: { active_users: "2410", active_workers: "86080", network_difficulty: "1", current_estimated_block_reward: "3.2" } });
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
    // payout row: 12,345 sats
    expect(await screen.findByText(/12,345 sats/)).toBeInTheDocument();
    expect(screen.getByText("2.00")).toBeInTheDocument(); // Th/s (5m, from statsnap)
    // user_hashrate wiring: live worker count + 1h average rendered.
    expect(screen.getByText("Workers")).toBeInTheDocument();
    expect(screen.getByText("3")).toBeInTheDocument(); // active_worker_count
    expect(screen.getByText(/1\.50 Th\/s/)).toBeInTheDocument(); // hashrate_3600s
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
    // stats still rendered (statsnap was fine)
    expect(screen.getByText("100,000")).toBeInTheDocument();
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
