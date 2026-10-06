import { describe, it, expect, vi } from "vitest";
import { LandfallClient } from "./api";

function jsonResponse(status: number, body: unknown): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "content-type": "application/json" },
  });
}

describe("LandfallClient", () => {
  it("parses a successful body", async () => {
    globalThis.fetch = vi.fn().mockResolvedValue(jsonResponse(200, { mnemonic: "a b", mining_address: "bc1qx" }));
    const r = await new LandfallClient("http://x", "tok").generate();
    expect(r.mining_address).toBe("bc1qx");
  });

  it("maps a server {error} body + status to ApiError", async () => {
    globalThis.fetch = vi.fn().mockResolvedValue(jsonResponse(409, { error: "seed exists" }));
    await expect(new LandfallClient("http://x", "tok").generate()).rejects.toMatchObject({
      status: 409,
      message: "seed exists",
    });
  });

  it("surfaces a network failure as status 0", async () => {
    globalThis.fetch = vi.fn().mockRejectedValue(new TypeError("connection refused"));
    await expect(new LandfallClient("http://x", "tok").generate()).rejects.toMatchObject({ status: 0 });
  });

  it("sends the bearer token, JSON content-type, and omits empty optionals", async () => {
    const f = vi.fn().mockResolvedValue(jsonResponse(200, { offer: "lno1" }));
    globalThis.fetch = f;
    await new LandfallClient("http://127.0.0.1:7762", "tok").offer("my desc");
    const [url, init] = f.mock.calls[0] as [string, RequestInit];
    expect(url).toBe("http://127.0.0.1:7762/offer");
    expect((init.headers as Record<string, string>).Authorization).toBe("Bearer tok");
    const body = JSON.parse(init.body as string);
    expect(body.description).toBe("my desc");
    expect(body.min_amount).toBeUndefined();
  });

  it("status GETs /status with the bearer token", async () => {
    const f = vi.fn().mockResolvedValue(jsonResponse(200, { configured: true, mining_address: "bc1qx", offer: "lno1x" }));
    globalThis.fetch = f;
    const s = await new LandfallClient("http://x", "tok").status();
    expect(s.configured).toBe(true);
    expect(s.mining_address).toBe("bc1qx");
    const [url, init] = f.mock.calls[0] as [string, RequestInit];
    expect(url).toBe("http://x/status");
    expect((init.headers as Record<string, string>).Authorization).toBe("Bearer tok");
  });

  it("payout posts message + offer", async () => {
    const f = vi.fn().mockResolvedValue(jsonResponse(200, { address: "bc1q", offer: "lno1", message: "m", signature: "sig" }));
    globalThis.fetch = f;
    const r = await new LandfallClient("http://x", "tok").payout("the message", "lno1");
    const body = JSON.parse((f.mock.calls[0][1] as RequestInit).body as string);
    expect(body).toEqual({ message: "the message", offer: "lno1" });
    expect(r.signature).toBe("sig");
  });

  it("revealSeed POSTs /seed/reveal with the bearer token", async () => {
    const f = vi.fn().mockResolvedValue(jsonResponse(200, { mnemonic: "w1 w2 w3" }));
    globalThis.fetch = f;
    const r = await new LandfallClient("http://x", "tok").revealSeed();
    expect(r.mnemonic).toBe("w1 w2 w3");
    const [url, init] = f.mock.calls[0] as [string, RequestInit];
    expect(url).toBe("http://x/seed/reveal");
    expect(init.method).toBe("POST");
    expect((init.headers as Record<string, string>).Authorization).toBe("Bearer tok");
  });

  it("nodeStatus GETs /node", async () => {
    const f = vi.fn().mockResolvedValue(jsonResponse(200, { node_pk: "02ab", lightning_total_sats: 1000, onchain_total_sats: 2000 }));
    globalThis.fetch = f;
    const s = await new LandfallClient("http://x", "tok").nodeStatus();
    expect((f.mock.calls[0][0] as string)).toBe("http://x/node");
    expect(s.lightning_total_sats).toBe(1000);
  });

  it("activity GETs /activity with the limit query", async () => {
    const f = vi.fn().mockResolvedValue(jsonResponse(200, [{ id: "a1", direction: "in", is_ocean: true }]));
    globalThis.fetch = f;
    const a = await new LandfallClient("http://x", "tok").activity(500);
    expect((f.mock.calls[0][0] as string)).toBe("http://x/activity?limit=500");
    expect(a[0].is_ocean).toBe(true);
  });

  it("createInvoice posts amount + description and returns the bolt11", async () => {
    const f = vi.fn().mockResolvedValue(jsonResponse(200, { invoice: "lnbc1abc" }));
    globalThis.fetch = f;
    const inv = await new LandfallClient("http://x", "tok").createInvoice(1234, "node wallet");
    expect((f.mock.calls[0][0] as string)).toBe("http://x/invoice");
    const body = JSON.parse((f.mock.calls[0][1] as RequestInit).body as string);
    expect(body).toEqual({ amount_sats: 1234, description: "node wallet" });
    expect(inv).toBe("lnbc1abc");
  });

  it("pay posts the payable + amount + note", async () => {
    const f = vi.fn().mockResolvedValue(jsonResponse(200, { id: "pid", amount_sats: 500, created_at_ms: 1 }));
    globalThis.fetch = f;
    const r = await new LandfallClient("http://x", "tok").pay("lnbc1dest", 500, "coffee");
    expect((f.mock.calls[0][0] as string)).toBe("http://x/pay");
    const body = JSON.parse((f.mock.calls[0][1] as RequestInit).body as string);
    expect(body).toEqual({ payable: "lnbc1dest", amount_sats: 500, note: "coffee" });
    expect(r.id).toBe("pid");
  });
});
