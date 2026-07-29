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

use oceanln_common::client;
use oceanln_common::error::Error;
use oceanln_common::seed::SeedSource;
use oceanln_common::service;
use oceanln_common::sign::DEFAULT_BIP32_PATH;
use oceanln_common::wallet_provider::{LexeWalletProvider, WalletProvider};
use lightning_invoice::Bolt11Invoice;

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
        })
    }

    // ── Offline / seed ops (sync) ──
    /// Whether a wallet seed is already configured (skip onboarding on launch).
    pub fn status(&self) -> CoreResult<StatusResp> {
        Ok(service::status(&self.seed, &self.default_path)?.into())
    }

    /// Generate a fresh 24-word seed, persist it, and return the phrase once.
    pub fn generate(&self) -> CoreResult<GenerateResp> {
        Ok(service::generate(&self.seed, &self.default_path)?.into())
    }

    /// Import an existing 24-word phrase. `force` overwrites an existing seed.
    pub fn import_seed(&self, mnemonic: String, force: bool) -> CoreResult<ImportResp> {
        let mut mnemonic = mnemonic;
        Ok(service::import(&self.seed, &self.default_path, &mut mnemonic, force)?.into())
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
        Ok(invoice.amount_milli_satoshis().map(|msat| msat.div_ceil(1000)))
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
