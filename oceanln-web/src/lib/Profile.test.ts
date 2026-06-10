import { render, screen, fireEvent } from "@testing-library/svelte";
import { describe, it, expect, beforeEach, vi } from "vitest";
import Profile from "./Profile.svelte";
import * as S from "./store.svelte";

const ADDR = "bc1qpstw48j7j9gjugw25jmjvd96jlwgdnedk5pr6r";
const res = (b: unknown) =>
  new Response(JSON.stringify(b), { status: 200, headers: { "content-type": "application/json" } });

beforeEach(() => {
  S.restart();
  S.app.miningAddress = ADDR;
  S.app.offer = "lno1testoffer";
  S.app.offerDescription = `OCEAN Payouts for ${ADDR}`;
  S.seedProfile(); // Profile derives from app.profile (1→n addresses + offers)
  globalThis.fetch = vi.fn(async (url: string | URL | Request) => {
    const u = String(url);
    if (u.includes("/statsnap/"))
      return res({ result: { snap_ts: "1700000000", hashrate_60s: "0", hashrate_300s: "2000000000000", shares_in_tides: "262144", estimated_payout_next_block: "0.0002", unpaid: "0.001", lastest_share_ts: "1700000000" } });
    if (u.includes("/user_hashrate/"))
      return res({ result: { snap_ts: "1700000000", db_ts: "1700000000", hashrate_60s: "0", hashrate_300s: "2000000000000", hashrate_600s: "0", hashrate_1800s: "0", hashrate_3600s: "0", hashrate_10800s: "0", hashrate_43200s: "0", hashrate_86400s: "0", active_worker_count: 2, lastest_share_ts: "1700000000" } });
    if (u.includes("/earnpay/"))
      return res({ result: { start_ts: 0, end_ts: 0, earnings: [], payouts: [{ ts: "1700000000", on_chain_txid: "abcdef1234567890", total_satoshis_net_paid: 12345, is_generation_txn: false }] } });
    if (u.includes("/pool_stat")) return res({ result: { active_users: "2410", active_workers: "1", network_difficulty: "1", current_estimated_block_reward: "3.2" } });
    return new Response("not found", { status: 404 });
  }) as typeof fetch;
});

describe("Profile — stats + profile in one view", () => {
  it("renders live OCEAN payout stats above the profile sections", async () => {
    render(Profile);
    // Embedded stats: title + a real OCEAN value (unpaid 0.001 BTC = 100,000 sats).
    expect(screen.getByText("Your payout stats")).toBeInTheDocument();
    expect(await screen.findByText("100,000")).toBeInTheDocument();
    expect(screen.getByText("Hashrate (5m)")).toBeInTheDocument();
    expect(screen.getByText("Workers")).toBeInTheDocument(); // from user_hashrate
    // Profile content present in the same view.
    expect(screen.getByText(/Onchain payout addresses/i)).toBeInTheDocument();
    expect(screen.getByText(/Recovery phrase/i)).toBeInTheDocument();
    // And a way to open the full Lightning dashboard (offer + MCP live there).
    expect(screen.getByText(/Open full Lightning dashboard/i)).toBeInTheDocument();
  });
});

describe("Profile — recovery-phrase reveal (issue #17)", () => {
  // The beforeEach leaves `app.phrase` empty — exactly the relaunched-session
  // state the bootstrap lands on (seed + offer on the server, no words in JS).
  it("fetches the stored phrase from the backend when it isn't held in session", async () => {
    const words = Array.from({ length: 24 }, (_, i) => `word${i + 1}`).join(" ");
    const statsFetch = globalThis.fetch;
    globalThis.fetch = vi.fn(async (url: string | URL | Request, init?: RequestInit) => {
      if (String(url).includes("/seed/reveal")) return res({ mnemonic: words });
      return statsFetch(url as never, init as never);
    }) as typeof fetch;
    render(Profile);
    await fireEvent.click(screen.getByRole("button", { name: /Reveal/ }));
    expect(await screen.findByText("word1")).toBeInTheDocument();
    expect(screen.getByText("word24")).toBeInTheDocument();
    // And the words landed in the shared store for this session.
    expect(S.app.phrase).toHaveLength(24);
  });

  it("surfaces the server's refusal instead of a dead-end", async () => {
    globalThis.fetch = vi.fn(async (url: string | URL | Request) => {
      if (String(url).includes("/seed/reveal"))
        return new Response(JSON.stringify({ error: "this endpoint requires authentication" }), {
          status: 403,
          headers: { "content-type": "application/json" },
        });
      return new Response("not found", { status: 404 });
    }) as typeof fetch;
    render(Profile);
    await fireEvent.click(screen.getByRole("button", { name: /Reveal/ }));
    expect(
      await screen.findByText(/Couldn't load your recovery phrase — this endpoint requires authentication/),
    ).toBeInTheDocument();
  });

  it("reveals straight from session memory when the phrase is already held", async () => {
    S.app.phrase = Array.from({ length: 24 }, (_, i) => `held${i + 1}`);
    const f = vi.fn(async (..._args: unknown[]) => new Response("not found", { status: 404 }));
    globalThis.fetch = f as typeof fetch;
    render(Profile);
    await fireEvent.click(screen.getByRole("button", { name: /Reveal/ }));
    expect(await screen.findByText("held1")).toBeInTheDocument();
    // No /seed/reveal round-trip when the words are already in memory.
    expect(f.mock.calls.map((c) => String(c[0]))).not.toContainEqual(expect.stringContaining("/seed/reveal"));
  });
});
