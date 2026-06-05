import { describe, it, expect, vi } from "vitest";
import { OceanlnClient } from "./api";

function jsonResponse(status: number, body: unknown): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "content-type": "application/json" },
  });
}

describe("OceanlnClient", () => {
  it("parses a successful body", async () => {
    globalThis.fetch = vi.fn().mockResolvedValue(jsonResponse(200, { mnemonic: "a b", mining_address: "bc1qx" }));
    const r = await new OceanlnClient("http://x", "tok").generate();
    expect(r.mining_address).toBe("bc1qx");
  });

  it("maps a server {error} body + status to ApiError", async () => {
    globalThis.fetch = vi.fn().mockResolvedValue(jsonResponse(409, { error: "seed exists" }));
    await expect(new OceanlnClient("http://x", "tok").generate()).rejects.toMatchObject({
      status: 409,
      message: "seed exists",
    });
  });

  it("surfaces a network failure as status 0", async () => {
    globalThis.fetch = vi.fn().mockRejectedValue(new TypeError("connection refused"));
    await expect(new OceanlnClient("http://x", "tok").generate()).rejects.toMatchObject({ status: 0 });
  });

  it("sends the bearer token, JSON content-type, and omits empty optionals", async () => {
    const f = vi.fn().mockResolvedValue(jsonResponse(200, { offer: "lno1" }));
    globalThis.fetch = f;
    await new OceanlnClient("http://127.0.0.1:7762", "tok").offer("my desc");
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
    const s = await new OceanlnClient("http://x", "tok").status();
    expect(s.configured).toBe(true);
    expect(s.mining_address).toBe("bc1qx");
    const [url, init] = f.mock.calls[0] as [string, RequestInit];
    expect(url).toBe("http://x/status");
    expect((init.headers as Record<string, string>).Authorization).toBe("Bearer tok");
  });

  it("payout posts message + offer", async () => {
    const f = vi.fn().mockResolvedValue(jsonResponse(200, { address: "bc1q", offer: "lno1", message: "m", signature: "sig" }));
    globalThis.fetch = f;
    const r = await new OceanlnClient("http://x", "tok").payout("the message", "lno1");
    const body = JSON.parse((f.mock.calls[0][1] as RequestInit).body as string);
    expect(body).toEqual({ message: "the message", offer: "lno1" });
    expect(r.signature).toBe("sig");
  });
});
