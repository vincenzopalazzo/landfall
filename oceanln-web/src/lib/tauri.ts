// Desktop transport: when running inside the Tauri shell, the wizard reaches the
// Rust backend over native IPC (`invoke`) instead of HTTP. No base URL, no bearer
// token — the commands run in-process. Selected by `store.svelte.ts#client()`.

import { invoke } from "@tauri-apps/api/core";
import {
  ApiError,
  type Backend,
  type GenerateResp,
  type ImportResp,
  type OfferResp,
  type InitResp,
  type PayoutResp,
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
}

/// A single Lightning payout to our BOLT12 offer, read straight from the
/// user's in-process Lexe wallet via the `list_lightning_payouts` IPC
/// command. NOT scraped from ocean.xyz — the truth source is the user's
/// own node.
export interface LightningPayoutRow {
  /// Lexe `PaymentId` (`<kind>_<hex>`) — carried for uniqueness; not used
  /// directly in URLs (use `payment_hash` for that).
  id: string;
  /// 32-byte Lightning payment hash, lowercase hex. Use it to build
  /// `https://ocean.xyz/info/tx/lightning/<hash>` for the explorer link.
  payment_hash: string | null;
  /// Net sats received, **rounded to the nearest whole sat**. Useful for
  /// display ("1 sat"); for sub-sat precision use [`amount_msat`].
  amount_sats: number;
  /// Net msats received — exact, no rounding. LN amounts are msat-granular.
  amount_msat: number;
  /// BOLT12 payer-supplied message — OCEAN puts the block height + hash
  /// here, so we surface it next to the row.
  payer_note: string | null;
  /// Payer's self-reported name (e.g. "Ocean Pool"), useful as a sanity
  /// check the payment really came from OCEAN.
  payer_name: string | null;
  /// Epoch milliseconds — `finalized_at` if set, else `created_at`.
  finalized_at_ms: number;
  /// Bitcoin block hash this payout settled (parsed from `payer_note`).
  /// Lowercase 64-char hex.
  block_hash: string;
  /// Bitcoin block height this payout settled (parsed from `payer_note`).
  block_height: number;
}

/// Tauri-only: list inbound BOLT12 offer payments from the user's Lexe
/// wallet. Returns an empty list (not throws) when not running in Tauri or
/// when the IPC fails — the dashboard then shows whatever `/v1/earnpay`
/// returns (typically nothing for Lightning, since OCEAN's public JSON
/// API omits them entirely).
export async function fetchLightningPayouts(): Promise<LightningPayoutRow[]> {
  if (!isTauri()) return [];
  try {
    return await invoke<LightningPayoutRow[]>("list_lightning_payouts");
  } catch {
    // Wallet read failure (node unreachable, mid-provision, etc.) shouldn't
    // blank the whole dashboard. Onchain payouts still come from the
    // separate /v1/earnpay fetch.
    return [];
  }
}
