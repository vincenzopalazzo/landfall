// Typed client for oceanln-httpd. Every seed-touching call carries the bearer
// token; errors surface the server's `{ "error": "…" }` body + HTTP status.

export class ApiError extends Error {
  status: number;
  constructor(status: number, message: string) {
    super(message);
    this.status = status;
    this.name = "ApiError";
  }
}

export interface GenerateResp {
  mnemonic: string;
  mining_address: string;
}
export interface ImportResp {
  mining_address: string;
}
export interface OfferResp {
  offer: string;
}
export interface InitResp {
  mining_address: string;
  provisioned: boolean;
}
export interface PayoutResp {
  address: string;
  offer: string;
  message: string;
  signature: string;
}
export interface StatusResp {
  configured: boolean;
  mining_address?: string | null;
  offer?: string | null;
}

/// One inbound OCEAN payout to the wallet's BOLT12 offer.
/// Wire-shape mirrors `oceanln_common::lexe_wallet::OceanPayout` — the
/// SAME struct the HTTP `GET /payouts` route and the Tauri
/// `list_lightning_payouts` IPC return. The frontend never duplicates
/// filtering or normalization — that all happens in Rust.
export interface OceanPayout {
  /// Lexe `PaymentId` (`<kind>_<hex>`). Carried for uniqueness; not in URLs.
  id: string;
  /// 32-byte Lightning payment hash, lowercase hex. Used to build the
  /// `ocean.xyz/info/tx/lightning/<hash>` deep link.
  payment_hash: string | null;
  /// Net sats received, rounded to the nearest whole sat.
  amount_sats: number;
  /// Net msats received — exact wire amount, no rounding. LN is msat-granular.
  amount_msat: number;
  /// BOLT12 payer-supplied note OCEAN signs into the invoice
  /// (e.g. `OCEAN lightning payout running at block <hash> at height <h>`).
  payer_note: string | null;
  /// Payer's self-reported name (often unset for OCEAN).
  payer_name: string | null;
  /// Epoch milliseconds — `finalized_at` if set, else `created_at`.
  finalized_at_ms: number;
  /// Block hash parsed from `payer_note`. Lowercase 64-char hex.
  block_hash: string;
  /// Block height parsed from `payer_note`.
  block_height: number;
}

/// The operations the wizard needs, independent of transport. The browser uses
/// `OceanlnClient` (HTTP → oceanln-httpd); the Tauri desktop shell uses
/// `TauriClient` (native IPC). `store.svelte.ts#client()` picks one at runtime.
export interface Backend {
  health(): Promise<boolean>;
  status(): Promise<StatusResp>;
  generate(): Promise<GenerateResp>;
  importSeed(mnemonic: string, force?: boolean): Promise<ImportResp>;
  offer(description?: string, minAmount?: string): Promise<OfferResp>;
  init(): Promise<InitResp>;
  payout(message: string, offer: string): Promise<PayoutResp>;
  /// List OCEAN payouts. Same Rust function runs behind both transports;
  /// the result shape is identical.
  payouts(limit?: number): Promise<OceanPayout[]>;
}

export class OceanlnClient implements Backend {
  constructor(
    private base: string,
    private token: string,
  ) {}

  private async post<T>(path: string, body: unknown): Promise<T> {
    let resp: Response;
    try {
      resp = await fetch(this.base + path, {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          Authorization: `Bearer ${this.token}`,
        },
        body: JSON.stringify(body ?? {}),
      });
    } catch (e) {
      throw new ApiError(0, `cannot reach the server at ${this.base} — is oceanln-httpd running?`);
    }
    return this.parse<T>(resp);
  }

  private async parse<T>(resp: Response): Promise<T> {
    const text = await resp.text();
    if (!resp.ok) {
      let msg = text;
      try {
        msg = JSON.parse(text).error ?? text;
      } catch {
        /* non-JSON error body */
      }
      throw new ApiError(resp.status, msg || `HTTP ${resp.status}`);
    }
    return (text ? JSON.parse(text) : {}) as T;
  }

  private async get<T>(path: string): Promise<T> {
    let resp: Response;
    try {
      resp = await fetch(this.base + path, {
        headers: { Authorization: `Bearer ${this.token}` },
      });
    } catch (e) {
      throw new ApiError(0, `cannot reach the server at ${this.base} — is oceanln-httpd running?`);
    }
    return this.parse<T>(resp);
  }

  async health(): Promise<boolean> {
    try {
      const r = await fetch(this.base + "/health");
      return r.ok;
    } catch {
      return false;
    }
  }

  status(): Promise<StatusResp> {
    return this.get<StatusResp>("/status");
  }

  generate(): Promise<GenerateResp> {
    return this.post<GenerateResp>("/generate", {});
  }
  importSeed(mnemonic: string, force = false): Promise<ImportResp> {
    return this.post<ImportResp>("/import", { mnemonic, force });
  }
  offer(description?: string, minAmount?: string): Promise<OfferResp> {
    return this.post<OfferResp>("/offer", {
      description: description || undefined,
      min_amount: minAmount || undefined,
    });
  }
  init(): Promise<InitResp> {
    return this.post<InitResp>("/init", {});
  }
  payout(message: string, offer: string): Promise<PayoutResp> {
    return this.post<PayoutResp>("/payout", { message, offer });
  }
  payouts(limit?: number): Promise<OceanPayout[]> {
    const q = typeof limit === "number" ? `?limit=${limit}` : "";
    return this.get<OceanPayout[]>(`/payouts${q}`);
  }
}
