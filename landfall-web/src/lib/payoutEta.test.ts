import { describe, it, expect, vi } from "vitest";
import { payoutEta, poolBlockShare, tipHeight } from "./payoutEta";

describe("poolBlockShare", () => {
  it("divides the pool hashrate by the network hashrate implied by difficulty", () => {
    // OCEAN on 2026-10-31: 44.8 EH/s against difficulty 148.36T.
    expect(poolBlockShare(44.8e18, 148_364_561_003_867)).toBeCloseTo(0.042, 3);
  });
});

describe("payoutEta", () => {
  // Blocks, at OCEAN's ~4.2% share of the network.
  it("waits for a new OCEAN block, then 30 more, once the last payout ran", () => {
    expect(payoutEta(0.042, 30)).toEqual({ median: 75, p90: 148 });
    expect(payoutEta(0.042, 200)).toEqual({ median: 75, p90: 148 });
  });

  it("counts down from OCEAN's latest block", () => {
    expect(payoutEta(0.042, 0)).toEqual({ median: 50, p90: 115 });
    expect(payoutEta(0.042, 20)).toEqual({ median: 10, p90: 77 });
    expect(payoutEta(0.042, 29)).toEqual({ median: 1, p90: 1 });
  });

  it("takes exactly 30 blocks when OCEAN almost never finds one to reset the count", () => {
    expect(payoutEta(1e-4, 0)).toEqual({ median: 30, p90: 30 });
  });

  it("gives no estimate for inputs that make no sense", () => {
    expect(payoutEta(0, 0)).toBeNull();
    expect(payoutEta(1, 0)).toBeNull();
    expect(payoutEta(NaN, 0)).toBeNull();
    expect(payoutEta(0.042, -1)).toBeNull();
    expect(payoutEta(0.042, 2.5)).toBeNull();
    // A pool this large resets the count almost every block.
    expect(payoutEta(0.5, 0)).toBeNull();
  });
});

describe("tipHeight", () => {
  it("parses mempool.space's plain-text height", async () => {
    const f = vi.fn().mockResolvedValue(new Response("974009"));
    globalThis.fetch = f;
    await expect(tipHeight()).resolves.toBe(974009);
    expect(String(f.mock.calls[0][0])).toBe("https://mempool.space/api/blocks/tip/height");
  });

  it("throws on an error status or a body that isn't a height", async () => {
    globalThis.fetch = vi.fn().mockResolvedValue(new Response("974009", { status: 503 }));
    await expect(tipHeight()).rejects.toThrow(/tip height/);
    globalThis.fetch = vi.fn().mockResolvedValue(new Response(""));
    await expect(tipHeight()).rejects.toThrow(/tip height/);
  });
});
