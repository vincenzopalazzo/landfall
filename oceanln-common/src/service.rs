//! Transport-agnostic orchestration for the OCEAN payout flow.
//!
//! These functions hold the logic every frontend shares: resolve the offer, read
//! the seed locally, derive + BIP-322 sign, provision the wallet, and
//! generate/import/persist a seed. The axum handlers in `oceanln-httpd`, the
//! Tauri desktop commands, and the MCP tools in `oceanln-mcp` all call straight
//! into here, so the security-critical steps — the offer-must-be-in-message
//! check, the seed never crossing the wire, the signing key wiped right after
//! use, and the atomic `O_EXCL` seed create — live in exactly one audited place
//! rather than being duplicated per transport.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
// `Zeroize` is only used by `import` (gated below). Keep the use under
// the same flag so the thin-build doesn't emit an unused-import error.
#[cfg(feature = "lexe-sdk")]
use zeroize::Zeroize;

use crate::client::{CreateOfferReq, SidecarClient};
use crate::error::{Error, Result};
use crate::seed::SeedSource;
use crate::sign;
// `MnemonicSecret` is only used by the `lexe-sdk`-gated `import` fn.
#[cfg(feature = "lexe-sdk")]
use crate::sign::MnemonicSecret;
// The WalletProvider trait + its consumers (`create_offer`, `init`,
// `list_offer_payouts`) only exist when the in-process Lexe SDK is
// compiled in — see `wallet_provider.rs` for why the gating is
// trait-level rather than per-method.
#[cfg(feature = "lexe-sdk")]
use crate::wallet_provider::WalletProvider;

// ── response payloads (shared by every transport) ──────────────────

// `Deserialize` added so `oceanln-mcp` (and any other cross-process
// consumer) can parse these back from a REST response. The structs are
// the wire contract of `oceanln-httpd`.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PayoutResp {
    pub address: String,
    pub offer: String,
    pub message: String,
    pub signature: String,
}

// `Deserialize` added so `oceanln-mcp` (and any other cross-process
// consumer) can parse these back from a REST response. The structs are
// the wire contract of `oceanln-httpd`.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct OfferResp {
    pub offer: String,
}

// `Deserialize` added so `oceanln-mcp` (and any other cross-process
// consumer) can parse these back from a REST response. The structs are
// the wire contract of `oceanln-httpd`.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct InitResp {
    pub mining_address: String,
    pub provisioned: bool,
}

// `Deserialize` added so `oceanln-mcp` (and any other cross-process
// consumer) can parse these back from a REST response. The structs are
// the wire contract of `oceanln-httpd`.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GenerateResp {
    /// The freshly generated 24-word phrase — revealed exactly once.
    pub mnemonic: String,
    pub mining_address: String,
}

// `Deserialize` added so `oceanln-mcp` (and any other cross-process
// consumer) can parse these back from a REST response. The structs are
// the wire contract of `oceanln-httpd`.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ImportResp {
    pub mining_address: String,
}

// `Deserialize` added so `oceanln-mcp` (and any other cross-process
// consumer) can parse these back from a REST response. The structs are
// the wire contract of `oceanln-httpd`.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StatusResp {
    /// Whether a wallet seed is already configured (so a frontend can skip
    /// onboarding and go straight to the profile/dashboard on launch).
    pub configured: bool,
    /// Derived offline from the seed; `None` when not configured.
    pub mining_address: Option<String>,
    /// The persisted primary BOLT12 offer, if one was created.
    pub offer: Option<String>,
}

/// Map a shared [`Error`] to an HTTP-style status code.
///
/// Used by every transport — the axum error response, the Tauri command
/// error, and the MCP tool error mapping — so a given failure reports
/// the same status everywhere (a browser `409` for an existing wallet
/// is a desktop `409` too).
pub fn http_status(err: &Error) -> u16 {
    match err {
        Error::InvalidOffer(_)
        | Error::InvalidMnemonic(_)
        | Error::AddressNotP2wpkh(_)
        | Error::InvalidBip32Path(_) => 400,
        Error::SeedExists { .. } => 409,
        Error::SidecarUnreachable { .. } => 502,
        Error::Api { code, .. } => *code,
        _ => 500,
    }
}

// ── operations ─────────────────────────────────────────────────────

/// End-to-end payout: resolve the offer, then derive the address and BIP-322
/// sign the message with the seed read from the configured source. The offer is
/// resolved before the seed is touched; when an `offer` is supplied (offline
/// mode) it must be embedded in `message`, mirroring OCEAN's verification flow.
#[allow(clippy::too_many_arguments)]
pub async fn payout(
    seed: &SeedSource,
    default_path: &str,
    sidecar_url: &str,
    sidecar_credentials: Option<&str>,
    message: String,
    offer: Option<String>,
    description: Option<&str>,
    min_amount: Option<&str>,
    path_override: Option<&str>,
) -> Result<PayoutResp> {
    let path = sign::parse_bip32_path(path_override.unwrap_or(default_path))?;

    // 1. Resolve the offer first (the only network call, and only when creating).
    let offer = match offer {
        Some(offer) => {
            if !offer.starts_with("lno1") {
                return Err(Error::InvalidOffer(format!(
                    "expected a BOLT12 offer starting with 'lno1': {offer}"
                )));
            }
            if !message.contains(&offer) {
                return Err(Error::InvalidOffer(
                    "offer is not present in message; OCEAN's message must embed the \
                     offer it authorizes (wrong offer or stale message?)"
                        .to_string(),
                ));
            }
            offer
        }
        None => {
            let client = SidecarClient::new(
                sidecar_url.to_string(),
                sidecar_credentials.map(|s| s.to_string()),
            )?;
            client
                .create_offer(CreateOfferReq {
                    description,
                    min_amount,
                })
                .await?
                .offer
        }
    };

    // 2. Read the seed locally, derive the key once, sign, then wipe the key.
    let secret = seed.load()?;
    let mnemonic = sign::parse_mnemonic(&secret)?;
    let mut key = sign::derive_private_key(&mnemonic, &path)?;
    let address = sign::address_from_key(&key)?;
    let signature = sign::sign_bip322(&key, &address, &message)?;
    key.inner.non_secure_erase();

    Ok(PayoutResp {
        address,
        offer,
        message,
        signature,
    })
}

/// List OCEAN's Lightning payouts from the user's in-process Lexe wallet.
///
/// The single transport-neutral entry point for the payouts data: the
/// HTTP route (`oceanln_httpd::payouts`), the CLI subcommand
/// (`oceanln payouts`), the Tauri desktop IPC, and the MCP tool
/// (`oceanln_mcp::list_payouts`) all funnel through this. Filtering /
/// OCEAN-pattern matching happens once, inside
/// [`crate::lexe_wallet::list_offer_payouts`] — never duplicated at the
/// transport layer.
#[cfg(feature = "lexe-sdk")]
pub async fn list_offer_payouts(
    seed: &SeedSource,
    wallet: &dyn WalletProvider,
    limit: u16,
) -> Result<Vec<crate::lexe_wallet::OceanPayout>> {
    let secret = seed.load()?;
    wallet.list_offer_payouts(secret.as_str(), limit).await
}

/// Live node status + balances, for the dashboard "Node wallet" cards.
#[cfg(feature = "lexe-sdk")]
pub async fn node_status(
    seed: &SeedSource,
    wallet: &dyn WalletProvider,
) -> Result<crate::lexe_wallet::NodeStatus> {
    let secret = seed.load()?;
    wallet.node_status(secret.as_str()).await
}

/// The node's full payment activity (inbound + outbound, LN + on-chain).
#[cfg(feature = "lexe-sdk")]
pub async fn list_payments(
    seed: &SeedSource,
    wallet: &dyn WalletProvider,
    limit: u16,
) -> Result<Vec<crate::lexe_wallet::Activity>> {
    let secret = seed.load()?;
    wallet.list_payments(secret.as_str(), limit).await
}

/// Create a BOLT11 invoice to receive a payment (Receive flow).
#[cfg(feature = "lexe-sdk")]
pub async fn create_invoice(
    seed: &SeedSource,
    wallet: &dyn WalletProvider,
    amount_sats: Option<u64>,
    description: Option<&str>,
) -> Result<String> {
    let secret = seed.load()?;
    wallet
        .create_invoice(secret.as_str(), amount_sats, description)
        .await
}

/// Send a payment to any payable string (Send flow). **Moves real funds.**
#[cfg(feature = "lexe-sdk")]
pub async fn pay(
    seed: &SeedSource,
    wallet: &dyn WalletProvider,
    payable: &str,
    amount_sats: Option<u64>,
    note: Option<&str>,
) -> Result<crate::lexe_wallet::PaySummary> {
    let secret = seed.load()?;
    wallet
        .pay(secret.as_str(), payable, amount_sats, note)
        .await
}

/// Create a payable BOLT12 offer in-process from the configured seed.
#[cfg(feature = "lexe-sdk")]
pub async fn create_offer(
    seed: &SeedSource,
    wallet: &dyn WalletProvider,
    description: Option<&str>,
    min_amount: Option<&str>,
) -> Result<OfferResp> {
    let secret = seed.load()?;
    let offer = wallet
        .create_offer(secret.as_str(), description, min_amount)
        .await?;
    // Persist only the FIRST (onboarding) offer as the primary one OCEAN is
    // configured with, so a restart restores it. Don't overwrite it when the
    // user mints additional offers later (Profile "New offer"), or a restart
    // would restore a secondary offer as the primary payout offer.
    if read_offer(seed).is_none() {
        write_offer(seed, &offer);
    }
    Ok(OfferResp { offer })
}

/// Sibling of the seed file (e.g. `…/seed` → `…/seed.offer`). The offer is a
/// public payment destination, not a secret, so it needs no special perms.
fn offer_path(seed: &SeedSource) -> PathBuf {
    seed.path().with_extension("offer")
}

/// The persisted primary BOLT12 offer, if one was created.
pub fn read_offer(seed: &SeedSource) -> Option<String> {
    std::fs::read_to_string(offer_path(seed))
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

// Only the `lexe-sdk`-gated `create_offer` writes new offers; the thin
// sidecar-client build only ever reads via `read_offer`.
#[cfg(feature = "lexe-sdk")]
fn write_offer(seed: &SeedSource, offer: &str) {
    // Best-effort, and deliberately written with default perms (0644, not the
    // seed's 0600): a BOLT12 offer is a public payment destination, not a
    // secret, so world-readable is intentional. A write failure only means the
    // offer won't be auto-restored after a restart, not that anything breaks.
    let _ = std::fs::write(offer_path(seed), offer);
}

/// Offline wallet status: whether a seed is configured and, if so, the derived
/// mining address + persisted offer. No Lexe/network — safe to call on launch
/// so the frontend can skip onboarding when a wallet already exists.
///
/// `configured` means *a seed file exists*, not that the wallet has been
/// provisioned on Lexe. The wizard always provisions during setup, so the only
/// way to reach a configured-but-unprovisioned state is `/import` without a
/// later `/init`; such a wallet still reports `configured: true` (its address
/// derives fine offline) but will have no persisted offer.
pub fn status(seed: &SeedSource, default_path: &str) -> Result<StatusResp> {
    if !seed.path().exists() {
        return Ok(StatusResp {
            configured: false,
            mining_address: None,
            offer: None,
        });
    }
    let path = sign::parse_bip32_path(default_path)?;
    let secret = seed.load()?;
    let mnemonic = sign::parse_mnemonic(&secret)?;
    let mining_address = sign::derive_address(&mnemonic, &path)?;
    Ok(StatusResp {
        configured: true,
        mining_address: Some(mining_address),
        offer: read_offer(seed),
    })
}

/// Provision the onchain wallet for the configured seed and return the mining
/// address to register with OCEAN. Idempotent. Operates on the already-stored
/// seed — call [`generate`] or [`import`] first to create one.
#[cfg(feature = "lexe-sdk")]
pub async fn init(
    seed: &SeedSource,
    default_path: &str,
    wallet: &dyn WalletProvider,
    path_override: Option<&str>,
) -> Result<InitResp> {
    let path = sign::parse_bip32_path(path_override.unwrap_or(default_path))?;
    let secret = seed.load()?;
    let mnemonic = sign::parse_mnemonic(&secret)?;
    let mining_address = sign::derive_address(&mnemonic, &path)?;
    wallet.provision(secret.as_str()).await?;
    Ok(InitResp {
        mining_address,
        provisioned: true,
    })
}

/// Generate a fresh 24-word recovery phrase, persist it to the configured seed
/// file, derive the mining address, and reveal the phrase once.
///
/// Refuses with [`Error::SeedExists`] if a seed file already exists — there is
/// deliberately no `force`: generating a new phrase over an existing wallet
/// would irreversibly destroy it, so replacing a wallet must go through
/// [`import`]. The `exists()` pre-check fails before generating so a refusal
/// never strands a revealed phrase; the persisting write is itself atomic
/// (`O_EXCL`), which is the real no-clobber guard under concurrency.
///
/// Gated on `lexe-sdk` because it calls [`sign::store_seed`], which is
/// itself only available with the in-process wallet feature on.
#[cfg(feature = "lexe-sdk")]
pub fn generate(seed: &SeedSource, default_path: &str) -> Result<GenerateResp> {
    let dest = seed.path();
    if dest.exists() {
        return Err(Error::SeedExists {
            path: dest.display().to_string(),
        });
    }
    let path = sign::parse_bip32_path(default_path)?;
    let secret = sign::generate_mnemonic()?;
    let mnemonic = sign::parse_mnemonic(&secret)?;
    let mining_address = sign::derive_address(&mnemonic, &path)?;
    sign::store_seed(&secret, Some(dest), false)?;
    // Wipe any stale sibling `.offer` from a previous wallet. Reachable when
    // an operator manually deleted the seed file (or moved it to a new path
    // that already had an old `.offer` lying around) and then hits
    // `/generate`. Without this, `create_offer` later refuses to overwrite
    // the existing offer and the next bootstrap restores the OLD wallet's
    // offer for the freshly-generated seed — pointing OCEAN at funds we
    // don't control.
    let _ = std::fs::remove_file(offer_path(seed));
    Ok(GenerateResp {
        mnemonic: secret.as_str().to_string(),
        mining_address,
    })
}

/// Import an existing 24-word phrase: validate it, persist it to the configured
/// seed file (409 if one already exists unless `force`), and return the derived
/// mining address. `mnemonic_input` is wiped after a zeroizing copy is taken.
///
/// Gated on `lexe-sdk` for the same reason as [`generate`].
#[cfg(feature = "lexe-sdk")]
pub fn import(
    seed: &SeedSource,
    default_path: &str,
    mnemonic_input: &mut String,
    force: bool,
) -> Result<ImportResp> {
    let path = sign::parse_bip32_path(default_path)?;
    let secret = MnemonicSecret::from_input(mnemonic_input);
    // Hold a zeroizing copy now; wipe the caller's plaintext buffer. (Transport
    // buffers upstream are out of our control, but we don't keep a second copy.)
    mnemonic_input.zeroize();
    let mnemonic = sign::parse_mnemonic(&secret)?; // 400 on a non-24-word phrase
    let mining_address = sign::derive_address(&mnemonic, &path)?;
    // A forced overwrite that actually swaps in a *different* seed makes the
    // persisted offer (created for the previous wallet) stale — drop it so a
    // later `status()` can't pair the new address with the old offer. Detected
    // before the write, since `store_seed` is idempotent for an identical phrase.
    let replaced_seed = force && seed_changed(seed, secret.as_str());
    // A FRESH import into a path with no prior seed but a stale sibling
    // `.offer` (operator manually deleted the seed file; the .offer was
    // left behind) is the same hazard the generate() path guards: without
    // clearing it, `create_offer` would refuse to overwrite and the next
    // `status()` would pair the freshly-imported wallet with the old
    // offer. Capture "did the seed file exist before we wrote?" so we
    // can decide whether to wipe a stale .offer after the write.
    let seed_existed_before = seed.path().exists();
    sign::store_seed(&secret, Some(seed.path()), force)?;
    if replaced_seed || !seed_existed_before {
        let _ = std::fs::remove_file(offer_path(seed));
    }
    Ok(ImportResp { mining_address })
}

/// True when a seed file exists and its (whitespace-normalized) contents differ
/// from `new_phrase`. `false` when no seed exists (a first write, not a swap).
#[cfg(feature = "lexe-sdk")]
fn seed_changed(seed: &SeedSource, new_phrase: &str) -> bool {
    match std::fs::read_to_string(seed.path()) {
        Ok(existing) => existing.split_whitespace().collect::<Vec<_>>().join(" ") != new_phrase,
        Err(_) => false,
    }
}
