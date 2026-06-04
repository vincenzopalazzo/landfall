import { describe, it, expect, vi } from "vitest";
import { ocean, btcToSats, hashesToThs } from "./ocean";

function res(body: unknown, status = 200): Response {
  return new Response(JSON.stringify(body), { status, headers: { "content-type": "application/json" } });
}

describe("ocean client", () => {
  it("unwraps the {result} envelope", async () => {
    globalThis.fetch = vi.fn().mockResolvedValue(res({ result: { unpaid: "0.00004845", hashrate_300s: "0" } }));
    const s = await ocean.statsnap("bc1qx");
    expect(s.unpaid).toBe("0.00004845");
  });

  it("throws on an {error} body (e.g. unknown user)", async () => {
    globalThis.fetch = vi.fn().mockResolvedValue(res({ error: "No such user or user has no active workers" }));
    await expect(ocean.statsnap("bc1qx")).rejects.toThrow(/no such user/i);
  });

  it("earnpay returns the payouts array and URL-encodes the address", async () => {
    const f = vi.fn().mockResolvedValue(
      res({ result: { earnings: [], payouts: [{ ts: 1, on_chain_txid: "t", total_satoshis_net_paid: 100, is_generation_txn: false }] } }),
    );
    globalThis.fetch = f;
    const e = await ocean.earnpay("bc1qaddr");
    expect(e.payouts).toHaveLength(1);
    expect(String(f.mock.calls[0][0])).toContain("/earnpay/bc1qaddr");
  });

  it("converts BTC→sats and hashes/sec→Th/s", () => {
    expect(btcToSats("0.00004845")).toBe(4845);
    expect(hashesToThs("1000000000000")).toBe(1);
  });
});
