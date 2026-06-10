// Desktop transport: when running inside the Tauri shell, the wizard reaches the
// Rust backend over native IPC (`invoke`) instead of HTTP. No base URL, no bearer
// token — the commands run in-process. Selected by `store.svelte.ts#client()`.

import { invoke } from "@tauri-apps/api/core";
import {
  ApiError,
  type Activity,
  type Backend,
  type GenerateResp,
  type ImportResp,
  type NodeStatus,
  type OceanPayout,
  type OfferResp,
  type InitResp,
  type PaySummary,
  type PayoutResp,
  type RevealResp,
  type StatusResp,
} from "./api";

/// True when running inside the Tauri webview (v2 exposes `__TAURI_INTERNALS__`).
export function isTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

// Tauri commands return `Err(CommandError { status, message })`; `invoke` rejects
// with that serialized object. Map it back to `ApiError` so the store's status
// checks (notably 409 → "wallet exists") behave identically to the HTTP path.
async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(cmd, args);
  } catch (e) {
    if (e && typeof e === "object" && "status" in e) {
      const ce = e as { status: number; message?: string };
      throw new ApiError(ce.status, ce.message ?? `command ${cmd} failed`);
    }
    throw new ApiError(0, e instanceof Error ? e.message : String(e));
  }
}

export class TauriClient implements Backend {
  // In-process: the backend is available as soon as the window is up.
  health(): Promise<boolean> {
    return Promise.resolve(true);
  }
  status(): Promise<StatusResp> {
    return call<StatusResp>("wallet_status");
  }
  generate(): Promise<GenerateResp> {
    return call<GenerateResp>("generate");
  }
  importSeed(mnemonic: string, force = false): Promise<ImportResp> {
    return call<ImportResp>("import_seed", { mnemonic, force });
  }
  revealSeed(): Promise<RevealResp> {
    return call<RevealResp>("reveal_seed");
  }
  offer(description?: string, minAmount?: string): Promise<OfferResp> {
    // camelCase keys map to the Rust command's snake_case args (Tauri v2).
    return call<OfferResp>("create_offer", {
      description: description || null,
      minAmount: minAmount || null,
    });
  }
  init(): Promise<InitResp> {
    return call<InitResp>("init_wallet", { path: null });
  }
  payout(message: string, offer: string): Promise<PayoutResp> {
    return call<PayoutResp>("payout", { message, offer });
  }
  // `list_lightning_payouts` is the Tauri IPC name; the Rust handler delegates
  // to the same `oceanln_common::lexe_wallet::list_offer_payouts` the HTTP
  // route uses, so this returns the identical `OceanPayout[]` shape.
  // The Rust command now accepts an optional `limit` — forward whatever
  // the caller asks for (StatsGrid uses 10_000 for the lifetime aggregate).
  payouts(limit?: number): Promise<OceanPayout[]> {
    return call<OceanPayout[]>(
      "list_lightning_payouts",
      typeof limit === "number" ? { limit } : undefined,
    );
  }
  nodeStatus(): Promise<NodeStatus> {
    return call<NodeStatus>("node_status");
  }
  activity(limit?: number): Promise<Activity[]> {
    return call<Activity[]>(
      "list_payments",
      typeof limit === "number" ? { limit } : undefined,
    );
  }
  async createInvoice(amountSats?: number, description?: string): Promise<string> {
    // camelCase keys → the Rust command's snake_case args (Tauri v2).
    const r = await call<{ invoice: string }>("create_invoice", {
      amountSats: amountSats ?? null,
      description: description ?? null,
    });
    return r.invoice;
  }
  pay(payable: string, amountSats?: number, note?: string): Promise<PaySummary> {
    return call<PaySummary>("pay", {
      payable,
      amountSats: amountSats ?? null,
      note: note ?? null,
    });
  }
}
