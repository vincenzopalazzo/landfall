import { describe, it, expect, vi, beforeEach } from "vitest";

// Mock the Tauri IPC bridge so we can drive TauriClient without a real shell.
const invokeMock = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({ invoke: (...args: unknown[]) => invokeMock(...args) }));

import { TauriClient, isTauri } from "./tauri";
import { ApiError } from "./api";

beforeEach(() => invokeMock.mockReset());

describe("TauriClient (desktop IPC transport)", () => {
  it("isTauri() is false outside the Tauri webview (so the browser uses HTTP)", () => {
    expect(isTauri()).toBe(false);
  });

  it("health() is true in-process (no server to reach)", async () => {
    expect(await new TauriClient().health()).toBe(true);
  });

  it("invokes the matching command with camelCase args", async () => {
    invokeMock.mockResolvedValueOnce({ offer: "lno1x" });
    const r = await new TauriClient().offer("desc", "1000");
    expect(invokeMock).toHaveBeenCalledWith("create_offer", { description: "desc", minAmount: "1000" });
    expect(r.offer).toBe("lno1x");
  });

  it("maps a CommandError {status,message} rejection to ApiError (so 409 → walletExists works)", async () => {
    invokeMock.mockRejectedValueOnce({ status: 409, message: "seed exists" });
    const err = await new TauriClient()
      .generate()
      .catch((e) => e);
    expect(err).toBeInstanceOf(ApiError);
    expect(err.status).toBe(409);
    expect(err.message).toBe("seed exists");
  });

  it("status invokes wallet_status (skip-the-wizard check)", async () => {
    invokeMock.mockResolvedValueOnce({ configured: true, mining_address: "bc1qx", offer: "lno1x" });
    const s = await new TauriClient().status();
    expect(invokeMock.mock.calls[0][0]).toBe("wallet_status");
    expect(s.configured).toBe(true);
    expect(s.mining_address).toBe("bc1qx");
  });

  it("import_seed forwards the phrase + force flag", async () => {
    invokeMock.mockResolvedValueOnce({ mining_address: "bc1qxyz" });
    await new TauriClient().importSeed("word1 word2", true);
    expect(invokeMock).toHaveBeenCalledWith("import_seed", { mnemonic: "word1 word2", force: true });
  });

  it("nodeStatus invokes node_status", async () => {
    invokeMock.mockResolvedValueOnce({ node_pk: "02ab", lightning_total_sats: 7 });
    const s = await new TauriClient().nodeStatus();
    expect(invokeMock.mock.calls[0][0]).toBe("node_status");
    expect(s.lightning_total_sats).toBe(7);
  });

  it("activity invokes list_payments with the limit", async () => {
    invokeMock.mockResolvedValueOnce([{ id: "a1", is_ocean: false }]);
    await new TauriClient().activity(500);
    expect(invokeMock).toHaveBeenCalledWith("list_payments", { limit: 500 });
  });

  it("createInvoice invokes create_invoice with camelCase args + returns bolt11", async () => {
    invokeMock.mockResolvedValueOnce({ invoice: "lnbc1z" });
    const inv = await new TauriClient().createInvoice(42, "desc");
    expect(invokeMock).toHaveBeenCalledWith("create_invoice", { amountSats: 42, description: "desc" });
    expect(inv).toBe("lnbc1z");
  });

  it("pay invokes pay with camelCase args (moves real funds → must map exactly)", async () => {
    invokeMock.mockResolvedValueOnce({ id: "pid", amount_sats: 9, created_at_ms: 0 });
    await new TauriClient().pay("lnbc1dst", 9, "n");
    expect(invokeMock).toHaveBeenCalledWith("pay", { payable: "lnbc1dst", amountSats: 9, note: "n" });
  });
});
