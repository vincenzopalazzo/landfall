//! oceanln-mobile-core — the OCEAN Lightning shared core, exposed to native
//! mobile (iOS + Android) via UniFFI.
//!
//! Architecture (bitkey-style: one shared core, native UI per platform):
//! the Compose Multiplatform UI calls the generated Kotlin bindings, which call
//! this crate, which is a **thin adapter over `oceanln_common::service`** — the
//! exact same transport-agnostic flow the CLI, `oceanln-httpd`, the Tauri
//! desktop shell, and `oceanln-mcp` all use. The security-critical steps (seed
//! read locally, signing key wiped after use, offer-in-message check) live once,
//! in `oceanln-common`, and mobile inherits them unchanged.
//!
//! This mirrors `src-tauri/src/lib.rs` 1-1: every method here has a
//! `#[tauri::command]` twin there. The only differences are transport
//! mechanics — UniFFI records/objects instead of Tauri IPC serde.

use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;

use lightning_invoice::Bolt11Invoice;
use oceanln_common::client;
use oceanln_common::error::Error;
use oceanln_common::ocean::OceanClient;
use oceanln_common::price::PriceClient;
use oceanln_common::seed::SeedSource;
use oceanln_common::service;
use oceanln_common::sign::DEFAULT_BIP32_PATH;
use oceanln_common::wallet_provider::{LexeWalletProvider, WalletProvider};

uniffi::setup_scaffolding!();

// ── Error ────────────────────────────────────────────────────────────
//
// Mirrors the Tauri `CommandError`: an HTTP-style status code (so a mobile
// `409` for "wallet exists" behaves the same as the browser/desktop `409`)
// plus a human message. Never carries the seed.
#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum CoreError {
    // `message` collides with Kotlin's Throwable.message in UniFFI-generated
    // exception subclasses. Keep the human-readable detail under a distinct
    // field name so Android bindings compile without manual edits.
    #[error("{detail}")]
    Failed { status: u16, detail: String },
}

impl From<Error> for CoreError {
    fn from(e: Error) -> Self {
        CoreError::Failed {
            status: service::http_status(&e),
            detail: e.to_string(),
        }
    }
}

type CoreResult<T> = Result<T, CoreError>;

// ── DTO mirror records ───────────────────────────────────────────────
//
// UniFFI can't derive `Record` on types owned by another crate, so we mirror
// the `service` / `lexe_wallet` response structs here with `From` conversions.
// `usize` isn't a UniFFI type — the channel counts widen to `u32`.

#[derive(uniffi::Record)]
pub struct StatusResp {
    pub configured: bool,
    pub mining_address: Option<String>,
    pub offer: Option<String>,
}
impl From<service::StatusResp> for StatusResp {
    fn from(r: service::StatusResp) -> Self {
        Self {
            configured: r.configured,
            mining_address: r.mining_address,
            offer: r.offer,
        }
    }
}

#[derive(uniffi::Record)]
pub struct GenerateResp {
    pub mnemonic: String,
    pub mining_address: String,
}
impl From<service::GenerateResp> for GenerateResp {
    fn from(r: service::GenerateResp) -> Self {
        Self {
            mnemonic: r.mnemonic,
            mining_address: r.mining_address,
        }
    }
}

#[derive(uniffi::Record)]
pub struct ImportResp {
    pub mining_address: String,
}
impl From<service::ImportResp> for ImportResp {
    fn from(r: service::ImportResp) -> Self {
        Self {
            mining_address: r.mining_address,
        }
    }
}

#[derive(uniffi::Record)]
pub struct RevealResp {
    pub mnemonic: String,
}
impl From<service::RevealResp> for RevealResp {
    fn from(r: service::RevealResp) -> Self {
        Self {
            mnemonic: r.mnemonic,
        }
    }
}

#[derive(uniffi::Record)]
pub struct OfferResp {
    pub offer: String,
}
impl From<service::OfferResp> for OfferResp {
    fn from(r: service::OfferResp) -> Self {
        Self { offer: r.offer }
    }
}

#[derive(uniffi::Record)]
pub struct InitResp {
    pub mining_address: String,
    pub provisioned: bool,
}
impl From<service::InitResp> for InitResp {
    fn from(r: service::InitResp) -> Self {
        Self {
            mining_address: r.mining_address,
            provisioned: r.provisioned,
        }
    }
}

#[derive(uniffi::Record)]
pub struct PayoutResp {
    pub address: String,
    pub offer: String,
    pub message: String,
    pub signature: String,
}
impl From<service::PayoutResp> for PayoutResp {
    fn from(r: service::PayoutResp) -> Self {
        Self {
            address: r.address,
            offer: r.offer,
            message: r.message,
            signature: r.signature,
        }
    }
}

#[derive(uniffi::Record)]
pub struct NodeStatus {
    pub node_pk: String,
    pub num_channels: u32,
    pub num_usable_channels: u32,
    pub lightning_total_sats: u64,
    pub lightning_sendable_sats: u64,
    pub onchain_total_sats: u64,
    pub onchain_trusted_sats: u64,
    pub total_balance_sats: u64,
}
impl From<oceanln_common::lexe_wallet::NodeStatus> for NodeStatus {
    fn from(r: oceanln_common::lexe_wallet::NodeStatus) -> Self {
        Self {
            node_pk: r.node_pk,
            num_channels: r.num_channels as u32,
            num_usable_channels: r.num_usable_channels as u32,
            lightning_total_sats: r.lightning_total_sats,
            lightning_sendable_sats: r.lightning_sendable_sats,
            onchain_total_sats: r.onchain_total_sats,
            onchain_trusted_sats: r.onchain_trusted_sats,
            total_balance_sats: r.total_balance_sats,
        }
    }
}

#[derive(uniffi::Record)]
pub struct OceanPayout {
    pub id: String,
    pub payment_hash: Option<String>,
    pub amount_sats: u64,
    pub amount_msat: u64,
    pub payer_note: Option<String>,
    pub payer_name: Option<String>,
    pub finalized_at_ms: i64,
    pub block_hash: String,
    pub block_height: u64,
}
impl From<oceanln_common::lexe_wallet::OceanPayout> for OceanPayout {
    fn from(r: oceanln_common::lexe_wallet::OceanPayout) -> Self {
        Self {
            id: r.id,
            payment_hash: r.payment_hash,
            amount_sats: r.amount_sats,
            amount_msat: r.amount_msat,
            payer_note: r.payer_note,
            payer_name: r.payer_name,
            finalized_at_ms: r.finalized_at_ms,
            block_hash: r.block_hash,
            block_height: r.block_height,
        }
    }
}

#[derive(uniffi::Record)]
pub struct Activity {
    pub id: String,
    pub direction: String,
    pub rail: String,
    pub amount_sats: u64,
    pub amount_msat: u64,
    pub fee_sats: u64,
    pub status: String,
    pub note: Option<String>,
    pub counterparty: Option<String>,
    pub finalized_at_ms: i64,
    pub payment_hash: Option<String>,
    pub txid: Option<String>,
    pub is_ocean: bool,
    pub block_height: Option<u64>,
    pub preimage: Option<String>,
    pub invoice: Option<String>,
    pub offer: Option<String>,
}
impl From<oceanln_common::lexe_wallet::Activity> for Activity {
    fn from(r: oceanln_common::lexe_wallet::Activity) -> Self {
        Self {
            id: r.id,
            direction: r.direction,
            rail: r.rail,
            amount_sats: r.amount_sats,
            amount_msat: r.amount_msat,
            fee_sats: r.fee_sats,
            status: r.status,
            note: r.note,
            counterparty: r.counterparty,
            finalized_at_ms: r.finalized_at_ms,
            payment_hash: r.payment_hash,
            txid: r.txid,
            is_ocean: r.is_ocean,
            block_height: r.block_height,
            preimage: r.preimage,
            invoice: r.invoice,
            offer: r.offer,
        }
    }
}

#[derive(uniffi::Record)]
pub struct PaySummary {
    pub id: String,
    pub amount_sats: u64,
    pub created_at_ms: i64,
}
impl From<oceanln_common::lexe_wallet::PaySummary> for PaySummary {
    fn from(r: oceanln_common::lexe_wallet::PaySummary) -> Self {
        Self {
            id: r.id,
            amount_sats: r.amount_sats,
            created_at_ms: r.created_at_ms,
        }
    }
}

// ── Pool stats (ocean.xyz public API) ────────────────────────────────
//
// Every field below comes from a real endpoint on `api.ocean.xyz/v1` via
// `oceanln_common::ocean::OceanClient`. The derivations are 1-1 with the
// frontend's `oceanln-web/src/lib/StatsGrid.svelte`, so the mobile Pool screen
// and the web dashboard cannot drift apart.
//
// Deliberately absent, because OCEAN's public API does not expose them and we
// will not invent them: pool-wide hashrate, blocks-found count, last-block
// height/age, share reject rate, a per-worker list (only a live *count* exists),
// and a total-workers figure.
#[derive(uniffi::Record)]
pub struct PoolStats {
    // ── user_hashrate: hashes/sec over each window ──
    pub hashrate_300s: f64,
    pub hashrate_3600s: f64,
    pub hashrate_10800s: f64,
    pub hashrate_86400s: f64,
    /// Live worker count. OCEAN reports how many are *active*; there is no
    /// "total configured" figure, so the UI shows this alone.
    pub active_workers: u32,
    /// Unix seconds of the most recent accepted share (0 = never).
    pub last_share_ts: i64,

    // ── statsnap: balances, all converted BTC→sats ──
    pub unpaid_sats: u64,
    pub est_payout_next_block_sats: u64,
    pub est_earn_next_block_sats: u64,

    // ── earnpay: payout history ──
    pub total_paid_sats: u64,
    /// `unpaid + total_paid`, matching the dashboard's "Lifetime" stat.
    pub lifetime_sats: u64,

    // ── TIDES window share ──
    pub tides_shares: f64,
    pub pool_tides_shares: f64,
    /// Your share of the current TIDES window, as a percentage — the same
    /// figure as the "Share Log Percentage" column on ocean.xyz/stats.
    pub share_pct: f64,

    // ── pool_stat: pool-wide context ──
    pub pool_active_users: u64,
    pub pool_active_workers: u64,
    pub network_difficulty: f64,

    /// True when `user_hashrate`/`statsnap` reported this address as unknown —
    /// a valid state for an address that has never mined, and distinct from a
    /// fetch failure (which surfaces as an error instead).
    pub address_unknown: bool,
}

/// OCEAN returns every numeric field as a string, and sometimes as a JSON
/// number. Coerce defensively: a missing/garbage value reads as `0.0` rather
/// than poisoning a total with NaN. Mirrors `num()` in `ocean.ts`.
fn num(s: &str) -> f64 {
    let n: f64 = s.trim().parse().unwrap_or(0.0);
    if n.is_finite() {
        n
    } else {
        0.0
    }
}

/// `num()` over a `serde_json::Value` that may be a number or a quoted string.
fn num_value(v: &serde_json::Value) -> f64 {
    match v {
        serde_json::Value::Number(n) => n.as_f64().filter(|f| f.is_finite()).unwrap_or(0.0),
        serde_json::Value::String(s) => num(s),
        _ => 0.0,
    }
}

/// Decimal BTC string → whole sats. Mirrors `btcToSats()` in `ocean.ts`.
/// Negative or non-finite inputs clamp to 0 rather than wrapping on cast.
fn btc_to_sats(btc: &str) -> u64 {
    let sats = (num(btc) * 1e8).round();
    if sats.is_finite() && sats > 0.0 {
        sats as u64
    } else {
        0
    }
}

/// OCEAN answers "this address has never mined here" with an error string
/// rather than an empty result. That is a legitimate empty state, not a
/// failure, so it must not be reported to the user as a broken fetch.
fn is_no_such_user(e: &Error) -> bool {
    e.to_string().to_lowercase().contains("no such user")
}

// ── Core object ──────────────────────────────────────────────────────
//
// One instance per app process. Constructed with the platform's app-data
// directory: the seed file lives there (created `0600` on first generate/import
// by `oceanln-common`), exactly like the Tauri shell roots it in the OS
// app-data dir. Keychain/Keystore-backed seed storage is a follow-up — see the
// README; it slots in as a new `SeedSource` variant with no call-site changes.
#[derive(uniffi::Object)]
pub struct OceanlnCore {
    seed: SeedSource,
    default_path: String,
    wallet: Arc<dyn WalletProvider>,
    backup_marker: PathBuf,
    sidecar_url: String,
    sidecar_credentials: Option<String>,
    /// One per process, as `ocean.rs` documents: the inner `reqwest::Client`
    /// pools TCP/TLS sessions across the four calls a Pool refresh makes.
    ocean: OceanClient,
    /// Holds the 60s BTC/USD cache, so it must be shared, not per-call.
    price: PriceClient,
}

#[uniffi::export]
impl OceanlnCore {
    /// Build the core rooted at the platform app-data directory. The directory
    /// is created if missing; the seed file is `<dir>/seed`.
    #[uniffi::constructor]
    pub fn new(app_data_dir: String) -> Arc<Self> {
        let dir = PathBuf::from(app_data_dir);
        // Best-effort: a later seed write surfaces any real permission error.
        let _ = std::fs::create_dir_all(&dir);
        Arc::new(Self {
            seed: SeedSource::File(dir.join("seed")),
            backup_marker: dir.join("backup-confirmed"),
            default_path: DEFAULT_BIP32_PATH.to_string(),
            wallet: Arc::new(LexeWalletProvider),
            sidecar_url: client::DEFAULT_BASE_URL.to_string(),
            sidecar_credentials: None,
            ocean: OceanClient::default(),
            price: PriceClient::default(),
        })
    }

    // ── Offline / seed ops (sync) ──
    /// Whether a wallet seed is already configured (skip onboarding on launch).
    pub fn status(&self) -> CoreResult<StatusResp> {
        Ok(service::status(&self.seed, &self.default_path)?.into())
    }

    /// Generate a fresh 24-word seed, persist it, and return the phrase once.
    pub fn generate(&self) -> CoreResult<GenerateResp> {
        let resp = service::generate(&self.seed, &self.default_path)?;
        self.clear_backup_marker();
        Ok(resp.into())
    }

    /// Import an existing 24-word phrase. `force` overwrites an existing seed.
    pub fn import_seed(&self, mnemonic: String, force: bool) -> CoreResult<ImportResp> {
        let mut mnemonic = mnemonic;
        let resp = service::import(&self.seed, &self.default_path, &mut mnemonic, force)?;
        self.clear_backup_marker();
        Ok(resp.into())
    }

    /// Drop the backup acknowledgement whenever the seed it referred to is
    /// replaced.
    ///
    /// The marker means "the user wrote *this* phrase down". Letting it outlive
    /// its seed is a fund-loss trap: the next launch would see
    /// `configured && backup_confirmed`, skip onboarding entirely, and the user
    /// would never be shown the phrase for the wallet they now actually hold.
    /// Callers re-confirm right after (a restored phrase is already in hand).
    fn clear_backup_marker(&self) {
        // Best-effort: a marker we failed to remove only means the user is
        // asked to confirm a backup they already made, which is the safe way
        // for this to fail.
        let _ = std::fs::remove_file(&self.backup_marker);
    }

    /// Re-reveal the stored recovery phrase (user-initiated backup view).
    pub fn reveal_seed(&self) -> CoreResult<RevealResp> {
        Ok(service::reveal(&self.seed)?.into())
    }

    /// Whether the generated recovery phrase was explicitly confirmed.
    pub fn backup_confirmed(&self) -> bool {
        self.backup_marker.is_file()
    }

    /// Persist the recovery-backup acknowledgement without storing the phrase.
    pub fn confirm_backup(&self) -> CoreResult<()> {
        std::fs::write(&self.backup_marker, b"confirmed").map_err(Error::Io)?;
        Ok(())
    }

    /// Return a fixed BOLT11 amount in whole sats, rounded up from msats.
    /// Other payable kinds are amountless from the mobile UI's perspective.
    pub fn payable_amount_sats(&self, payable: String) -> CoreResult<Option<u64>> {
        let value = payable.trim();
        let lower = value.to_ascii_lowercase();
        if !["lnbc", "lntb", "lnbcrt", "lnsb"]
            .iter()
            .any(|prefix| lower.starts_with(prefix))
        {
            return Ok(None);
        }
        let invoice = Bolt11Invoice::from_str(value)
            .map_err(|e| Error::Wallet(format!("invalid BOLT11 invoice: {e}")))?;
        Ok(invoice
            .amount_milli_satoshis()
            .map(|msat| msat.div_ceil(1000)))
    }
}

#[cfg(test)]
mod tests {
    use super::OceanlnCore;

    #[test]
    fn backup_confirmation_survives_core_recreation() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().to_string_lossy().into_owned();
        let core = OceanlnCore::new(path.clone());
        assert!(!core.backup_confirmed());
        core.confirm_backup().expect("confirm backup");
        assert!(OceanlnCore::new(path).backup_confirmed());
    }

    #[test]
    fn replacing_the_seed_clears_the_backup_marker() {
        // Regression: a marker that outlives its seed makes the next launch
        // skip onboarding, so the user is never shown the phrase for the
        // wallet they now hold.
        let dir = tempfile::tempdir().expect("temp dir");
        let core = OceanlnCore::new(dir.path().to_string_lossy().into_owned());

        core.generate().expect("generate");
        core.confirm_backup().expect("confirm backup");
        assert!(core.backup_confirmed());

        // A different phrase replaces the seed; the old acknowledgement must go.
        let other = "abandon abandon abandon abandon abandon abandon abandon abandon \
                     abandon abandon abandon abandon abandon abandon abandon abandon \
                     abandon abandon abandon abandon abandon abandon abandon art";
        core.import_seed(other.to_string(), true).expect("import");
        assert!(
            !core.backup_confirmed(),
            "backup marker must not survive a seed replacement"
        );
    }

    #[test]
    fn num_coerces_ocean_string_fields() {
        assert_eq!(super::num("2000000000000"), 2e12);
        assert_eq!(super::num(" 0.00015 "), 0.00015);
        // Garbage/empty must read as 0, not NaN — a NaN here would poison
        // every total it touches and render as "NaN sats".
        assert_eq!(super::num(""), 0.0);
        assert_eq!(super::num("not-a-number"), 0.0);
        assert_eq!(super::num("NaN"), 0.0);
        assert_eq!(super::num("inf"), 0.0);
    }

    #[test]
    fn num_value_accepts_numbers_and_quoted_strings() {
        // OCEAN sends `total_satoshis_net_paid` both ways across endpoints.
        assert_eq!(super::num_value(&serde_json::json!(42)), 42.0);
        assert_eq!(super::num_value(&serde_json::json!("42")), 42.0);
        assert_eq!(super::num_value(&serde_json::json!(null)), 0.0);
        assert_eq!(super::num_value(&serde_json::json!("junk")), 0.0);
    }

    #[test]
    fn btc_to_sats_matches_the_frontend() {
        assert_eq!(super::btc_to_sats("0.00015"), 15_000);
        assert_eq!(super::btc_to_sats("1"), 100_000_000);
        assert_eq!(super::btc_to_sats("0"), 0);
        // Never wrap on cast: a negative or garbage amount clamps to 0.
        assert_eq!(super::btc_to_sats("-1"), 0);
        assert_eq!(super::btc_to_sats("junk"), 0);
    }

    #[test]
    fn no_such_user_is_recognised_as_an_empty_state() {
        use oceanln_common::error::Error;
        let e = Error::Wallet(
            "ocean.xyz https://api.ocean.xyz/v1/statsnap/bc1q returned error: \
             No such user or user has no active workers"
                .to_string(),
        );
        assert!(super::is_no_such_user(&e));
        assert!(!super::is_no_such_user(&Error::Wallet(
            "connection refused".to_string()
        )));
    }

    #[test]
    fn non_bolt11_payables_have_no_fixed_amount() {
        let dir = tempfile::tempdir().expect("temp dir");
        let core = OceanlnCore::new(dir.path().to_string_lossy().into_owned());
        for payable in ["alice@example.com", "lno1example", "lnurl1example"] {
            assert_eq!(
                core.payable_amount_sats(payable.to_string())
                    .expect("parse payable"),
                None
            );
        }
    }
}

#[uniffi::export(async_runtime = "tokio")]
impl OceanlnCore {
    /// Create a payable BOLT12 offer in-process from the configured seed.
    pub async fn create_offer(
        &self,
        description: Option<String>,
        min_amount: Option<String>,
    ) -> CoreResult<OfferResp> {
        Ok(service::create_offer(
            &self.seed,
            self.wallet.as_ref(),
            description.as_deref(),
            min_amount.as_deref(),
        )
        .await?
        .into())
    }

    /// Provision the on-chain wallet + derive the mining address.
    pub async fn init_wallet(&self, path_override: Option<String>) -> CoreResult<InitResp> {
        Ok(service::init(
            &self.seed,
            &self.default_path,
            self.wallet.as_ref(),
            path_override.as_deref(),
        )
        .await?
        .into())
    }

    /// Resolve OCEAN's offer, derive + BIP-322 sign the payout message.
    pub async fn payout(&self, message: String, offer: Option<String>) -> CoreResult<PayoutResp> {
        Ok(service::payout(
            &self.seed,
            &self.default_path,
            &self.sidecar_url,
            self.sidecar_credentials.as_deref(),
            message,
            offer,
            None,
            None,
            None,
        )
        .await?
        .into())
    }

    /// Live node status + balances (Pool/Wallet/Node screens).
    pub async fn node_status(&self) -> CoreResult<NodeStatus> {
        Ok(service::node_status(&self.seed, self.wallet.as_ref())
            .await?
            .into())
    }

    /// Full payment activity (inbound + outbound, LN + on-chain), newest first.
    pub async fn list_payments(&self, limit: u16) -> CoreResult<Vec<Activity>> {
        Ok(
            service::list_payments(&self.seed, self.wallet.as_ref(), limit)
                .await?
                .into_iter()
                .map(Into::into)
                .collect(),
        )
    }

    /// OCEAN payouts only (verified BOLT12-offer inbound payments).
    pub async fn list_offer_payouts(&self, limit: u16) -> CoreResult<Vec<OceanPayout>> {
        Ok(
            service::list_offer_payouts(&self.seed, self.wallet.as_ref(), limit)
                .await?
                .into_iter()
                .map(Into::into)
                .collect(),
        )
    }

    /// Create a BOLT11 invoice to receive (Receive flow). `None` = amountless.
    pub async fn create_invoice(
        &self,
        amount_sats: Option<u64>,
        description: Option<String>,
    ) -> CoreResult<String> {
        service::create_invoice(
            &self.seed,
            self.wallet.as_ref(),
            amount_sats,
            description.as_deref(),
        )
        .await
        .map_err(Into::into)
    }

    /// Live mining stats for `address` from OCEAN's public API.
    ///
    /// Four endpoints in parallel, each best-effort in the same way the web
    /// dashboard treats them (`StatsGrid.svelte`): `user_hashrate`, `earnpay`
    /// and `pool_stat` contribute zeros when they fail, because a brand-new
    /// address legitimately 404s on them.
    ///
    /// `statsnap` is the exception — if *it* fails for any reason other than
    /// "no such user", the whole call errors. Returning a screen full of
    /// plausible zeros for what is really a dead network is the failure mode
    /// this app can least afford.
    pub async fn pool_stats(&self, address: String) -> CoreResult<PoolStats> {
        let (snap, hashrate, earnpay, pool) = tokio::join!(
            self.ocean.statsnap(&address),
            self.ocean.user_hashrate(&address),
            self.ocean.earnpay(&address),
            self.ocean.pool_stat(),
        );

        let mut address_unknown = false;
        let snap = match snap {
            Ok(s) => Some(s),
            Err(e) if is_no_such_user(&e) => {
                address_unknown = true;
                None
            }
            Err(e) => return Err(e.into()),
        };

        // statsnap: balances + TIDES shares + the 5m hashrate window.
        let (unpaid_sats, est_payout_next_block_sats, est_earn_next_block_sats) = snap
            .as_ref()
            .map(|s| {
                (
                    btc_to_sats(&s.unpaid),
                    btc_to_sats(&s.estimated_payout_next_block),
                    btc_to_sats(&s.estimated_earn_next_block),
                )
            })
            .unwrap_or((0, 0, 0));
        let tides_shares = snap
            .as_ref()
            .map(|s| num(&s.shares_in_tides))
            .unwrap_or(0.0);
        let mut hashrate_300s = snap.as_ref().map(|s| num(&s.hashrate_300s)).unwrap_or(0.0);
        let mut last_share_ts = snap
            .as_ref()
            .map(|s| num(&s.lastest_share_ts) as i64)
            .unwrap_or(0);

        // user_hashrate: live worker count + the longer windows.
        let (mut active_workers, mut hashrate_3600s, mut hashrate_10800s, mut hashrate_86400s) =
            (0u32, 0.0, 0.0, 0.0);
        if let Ok(h) = hashrate {
            active_workers = h.active_worker_count.min(u32::MAX as u64) as u32;
            hashrate_3600s = num(&h.hashrate_3600s);
            hashrate_10800s = num(&h.hashrate_10800s);
            hashrate_86400s = num(&h.hashrate_86400s);
            // Prefer statsnap's 5m window, but take this one when statsnap was
            // the endpoint that had no record of the address.
            if hashrate_300s == 0.0 {
                hashrate_300s = num(&h.hashrate_300s);
            }
            last_share_ts = last_share_ts.max(num(&h.lastest_share_ts) as i64);
        }

        // earnpay: sum the payout history. We ignore the `earnings` array for
        // the same reason the web does — per-block credits duplicate what the
        // payouts list already implies.
        let total_paid_sats: u64 = earnpay
            .map(|e| {
                e.payouts
                    .iter()
                    .map(|p| {
                        let v = num_value(&p.total_satoshis_net_paid);
                        if v.is_finite() && v > 0.0 {
                            v as u64
                        } else {
                            0
                        }
                    })
                    .sum()
            })
            .unwrap_or(0);

        // pool_stat: pool-wide context + the denominator for the TIDES share.
        let (mut pool_tides_shares, mut pool_active_users, mut pool_active_workers, mut difficulty) =
            (0.0, 0u64, 0u64, 0.0);
        if let Ok(p) = pool {
            pool_tides_shares = num(&p.current_tides_shares);
            pool_active_users = num(&p.active_users) as u64;
            pool_active_workers = num(&p.active_workers) as u64;
            difficulty = num(&p.network_difficulty);
        }
        let share_pct = if pool_tides_shares > 0.0 {
            (tides_shares / pool_tides_shares) * 100.0
        } else {
            0.0
        };

        Ok(PoolStats {
            hashrate_300s,
            hashrate_3600s,
            hashrate_10800s,
            hashrate_86400s,
            active_workers,
            last_share_ts,
            unpaid_sats,
            est_payout_next_block_sats,
            est_earn_next_block_sats,
            total_paid_sats,
            lifetime_sats: unpaid_sats.saturating_add(total_paid_sats),
            tides_shares,
            pool_tides_shares,
            share_pct,
            pool_active_users,
            pool_active_workers,
            network_difficulty: difficulty,
            address_unknown,
        })
    }

    /// Current BTC/USD spot, or `0.0` when unavailable.
    ///
    /// `0` is not an error — it means "show sats". The UI must never render a
    /// fiat figure from a rate it does not actually have.
    pub async fn btc_usd(&self) -> f64 {
        self.price.btc_usd().await
    }

    /// Send to any payable string (Send flow). **Moves real funds.**
    pub async fn pay(
        &self,
        payable: String,
        amount_sats: Option<u64>,
        note: Option<String>,
    ) -> CoreResult<PaySummary> {
        Ok(service::pay(
            &self.seed,
            self.wallet.as_ref(),
            &payable,
            amount_sats,
            note.as_deref(),
        )
        .await?
        .into())
    }
}
