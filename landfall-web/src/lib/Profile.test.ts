import { render, screen, fireEvent, waitFor } from "@testing-library/svelte";
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

describe("Profile — next Lightning payout estimate", () => {
  const TIP_URL = "https://mempool.space/api/blocks/tip/height";
  // OCEAN's pool hashrate and two recent blocks (newest first), plus the chain tip.
  function mockPayoutInputs(tip: () => Response) {
    const statsFetch = globalThis.fetch;
    globalThis.fetch = vi.fn(async (url: string | URL | Request, init?: RequestInit) => {
      const u = String(url);
      if (u === TIP_URL) return tip();
      if (u.endsWith("/v1/pool_hashrate")) return res({ result: { snap_ts: 1, pool_60s: "0", pool_300s: "44800000000000000000" } });
      if (u.endsWith("/v1/blocks"))
        return res({
          result: {
            blocks: [
              { height: 974003, ts: "2026-10-31T17:36:23.958200", block_hash: "00", network_difficulty: 148364561003867 },
              { height: 973990, ts: "2026-10-31T15:10:05.331700", block_hash: "01", network_difficulty: 148364561003867 },
            ],
          },
        });
      return statsFetch(url as never, init as never);
    }) as typeof fetch;
  }

  it("counts the blocks since OCEAN's latest one and shows the expected wait", async () => {
    mockPayoutInputs(() => new Response("974009"));
    render(Profile);
    // 6 blocks into the 30-block count at OCEAN's ~4.2% share: median 43 blocks, 90% within 109.
    expect(
      await screen.findByText("Next Lightning payout: usually within ~7 h, 9 times in 10 within ~18 h."),
    ).toBeInTheDocument();
    expect(screen.getByText(/#974,003.*30 blocks after it \(6 so far\)/)).toBeInTheDocument();
  });

  it("hides the estimate, without an error banner, once an input starts failing", async () => {
    let tipUp = true;
    mockPayoutInputs(() => (tipUp ? new Response("974009") : new Response("unavailable", { status: 503 })));
    render(Profile);
    expect(await screen.findByText(/Next Lightning payout/)).toBeInTheDocument();
    tipUp = false;
    const refresh = screen.getByRole("button", { name: /Refresh/ });
    await waitFor(() => expect(refresh).not.toBeDisabled());
    await fireEvent.click(refresh);
    await waitFor(() => expect(screen.queryByText(/Next Lightning payout/)).not.toBeInTheDocument());
    expect(screen.queryByText(/Couldn't load OCEAN data/)).not.toBeInTheDocument();
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
