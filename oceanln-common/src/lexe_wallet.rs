//! In-process Lexe wallet operations (feature `lexe-sdk`).
//!
//! Replaces the sidecar HTTP path with direct `lexe` SDK calls, all driven
//! from the user's 24-word seed:
//! - [`init`] creates + provisions the onchain Lexe wallet (`signup`), and
//! - [`create_offer`] mints a payable BOLT12 offer on the node.
//!
//! Mirrors `lexe-cli`'s usage of the same SDK (`init` / `create-offer`).

use std::str::FromStr;

use lexe::config::WalletEnvConfig;
use lexe::types::auth::{CredentialsRef, RootSeed};
use lexe::types::bitcoin::Amount;
use lexe::types::command::CreateOfferRequest;
use lexe::wallet::LexeWallet;
use lexe_api_core::def::AppNodeRunApi;
use lexe_api_core::models::command::GetUpdatedPayments;
use lexe_api_core::types::payments::{
    PaymentDirection, PaymentKind, PaymentStatus, PaymentUpdatedIndex,
};
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

/// A single inbound offer payment to OUR BOLT12 offer, surfaced from the
/// in-process Lexe wallet (NOT scraped from any web UI). One row per Lightning
/// payout OCEAN sends to the user.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OceanPayout {
    /// Lexe's `PaymentId` serialized form: `<kind>_<hex>`. Carried for
    /// uniqueness / debugging; the *payment-hash* used in OCEAN's deep link is
    /// the separate [`Self::payment_hash`] field below.
    pub id: String,
    /// 32-byte Lightning payment hash, lowercase hex (no prefix). Used to
    /// build `https://ocean.xyz/info/tx/lightning/<hash>`. `None` if the
    /// upstream payment is still pending (we filter those out, so in
    /// practice this should always be `Some` on the wire).
    pub payment_hash: Option<String>,
    /// Net sats received, **rounded to the nearest whole sat** (the Lexe
    /// wallet's view — already accounts for any JIT-channel skim). LN
    /// amounts are millisat-granular, and OCEAN's per-block payouts are
    /// often sub-sat (e.g. 995 msats = ~1 sat); naive truncation would
    /// render a real 1-sat payout as 0. See [`Self::amount_msat`] for
    /// full precision. `0` if the upstream amount was unset.
    pub amount_sats: u64,
    /// Net msats received — the exact wire amount, no rounding. Useful
    /// when the frontend wants to show sub-sat precision (e.g. "0.995
    /// sats" instead of "1 sat"). `0` if the upstream amount was unset.
    pub amount_msat: u64,
    /// OCEAN encodes the block-height + block-hash this payout is settling
    /// in the BOLT12 invoice's payer-supplied message field. In
    /// `lexe-api-core` v0.1.14 this field is named `message` on the wire;
    /// we expose it here as `payer_note` because that's the BOLT12 spec
    /// term users will recognize.
    pub payer_note: Option<String>,
    /// The payer's self-reported name (OCEAN typically sets "Ocean Pool" or
    /// similar — useful as a sanity check that this really is an OCEAN payout).
    pub payer_name: Option<String>,
    /// When the payment was finalized (epoch milliseconds). Falls back to
    /// `created_at` if `finalized_at` is unset.
    pub finalized_at_ms: i64,
    /// The Bitcoin block this OCEAN payout settles (lower-case hex,
    /// 64 chars). Parsed from [`Self::payer_note`].
    pub block_hash: String,
    /// The Bitcoin block height this OCEAN payout settles. Parsed from
    /// [`Self::payer_note`].
    pub block_height: u64,
}

/// Parse OCEAN's BOLT12 payer-note format:
/// `OCEAN lightning payout running at block `<hash>` at height `<height>``
///
/// Returns `Some((block_hash, block_height))` only when the note matches the
/// exact OCEAN format (including the surrounding backticks and a 64-hex
/// block hash). Any other BOLT12 payer-note — including the user paying
/// themselves with a tab character — returns `None` and so is filtered out
/// of the OCEAN payouts list.
fn parse_ocean_payer_note(note: &str) -> Option<(String, u64)> {
    let rest = note.strip_prefix("OCEAN lightning payout running at block `")?;
    let (hash, rest) = rest.split_once("` at height `")?;
    let height_str = rest.strip_suffix('`')?;
    if hash.len() != 64 || !hash.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let height: u64 = height_str.parse().ok()?;
    Some((hash.to_ascii_lowercase(), height))
}

/// Parse a 24-word mnemonic into a Lexe `RootSeed`.
fn root_seed(mnemonic: &str) -> Result<RootSeed> {
    let m = lexe::bip39::Mnemonic::from_str(mnemonic.trim())
        .map_err(|e| Error::Wallet(format!("invalid mnemonic: {e}")))?;
    RootSeed::from_mnemonic(m).map_err(|e| Error::Wallet(format!("root seed: {e:#}")))
}

/// Build a stateless (`without_db`) mainnet wallet from the seed.
fn wallet(seed: &RootSeed) -> Result<LexeWallet> {
    LexeWallet::without_db(WalletEnvConfig::mainnet(), CredentialsRef::from(seed))
        .map_err(|e| Error::Wallet(format!("wallet init: {e:#}")))
}

/// Create + provision the onchain Lexe wallet for this seed.
///
/// Equivalent to `lexe init`: registers the user with Lexe's backend and runs
/// the initial provision. Idempotent — safe to call when already signed up.
pub async fn init(mnemonic: &str) -> Result<()> {
    let seed = root_seed(mnemonic)?;
    let wallet = wallet(&seed)?;
    wallet
        .signup(&seed, None)
        .await
        .map_err(|e| Error::Wallet(format!("signup/provision: {e:#}")))
}

/// Create a payable BOLT12 offer on the node; returns the `lno1…` string.
///
/// Requires the wallet to have been provisioned first (see [`init`]).
pub async fn create_offer(
    mnemonic: &str,
    description: Option<&str>,
    min_amount: Option<&str>,
) -> Result<String> {
    let seed = root_seed(mnemonic)?;
    let wallet = wallet(&seed)?;

    let min_amount = match min_amount {
        Some(s) => Some(
            Amount::from_str(s).map_err(|e| Error::Wallet(format!("invalid --min-amount: {e}")))?,
        ),
        None => None,
    };

    let resp = wallet
        .create_offer(CreateOfferRequest {
            description: description.map(String::from),
            min_amount,
            expiration_secs: None,
        })
        .await
        .map_err(|e| Error::Wallet(format!("create offer: {e:#}")))?;

    Ok(resp.offer.to_string())
}

/// List inbound, completed, offer-paid payments — i.e. OCEAN's Lightning
/// payouts to the user's BOLT12 offer.
///
/// Reads straight from the Lexe-hosted node (no local DB, no web scrape, no
/// trust in ocean.xyz beyond the bytes they signed and sent as a Lightning
/// payment). The filter is deliberately broad — `kind == Offer` AND
/// `direction == Inbound` AND `status == Completed` — so any future OCEAN
/// payment surfaces without code changes. If the user has multiple BOLT12
/// offers and only one is registered with OCEAN, *all* inbound offer
/// payments still show; in practice OCEAN is currently the only entity
/// paying mining addresses to BOLT12 reusable offers at scale.
///
/// `limit` is the cap on the number of OCEAN **payouts** returned (after
/// filtering), not on raw payments fetched. We keep paginating through the
/// wallet's payment history until we have `limit` OCEAN payouts or the
/// wallet has no more payments. A separate hard ceiling on total
/// payments scanned prevents a runaway loop if the user has a huge
/// non-OCEAN history (`MAX_PAYMENTS_SCANNED`).
const MAX_PAYMENTS_SCANNED: usize = 10_000;

pub async fn list_offer_payouts(mnemonic: &str, limit: u16) -> Result<Vec<OceanPayout>> {
    let seed = root_seed(mnemonic)?;
    let wallet = wallet(&seed)?;
    // Lexe caps a single request at 100 payments
    // (`lexe_common::constants::MAX_PAYMENTS_BATCH_SIZE` = 100). Paginate
    // using the response's last `(updated_at, id)` as the next
    // `start_index` (exclusive), exactly how `lexe::wallet::sync_payments`
    // does it.
    const PER_BATCH: u16 = 100;
    let mut start_index: Option<PaymentUpdatedIndex> = None;
    let mut out: Vec<OceanPayout> = Vec::new();
    let limit_total = usize::from(limit);
    let mut scanned: usize = 0;

    loop {
        // Cap on FILTERED rows — what the caller actually asked for.
        if out.len() >= limit_total {
            break;
        }
        // Safety valve against a runaway loop on a huge wallet history
        // where almost nothing matches the OCEAN filter.
        if scanned >= MAX_PAYMENTS_SCANNED {
            break;
        }
        let req = GetUpdatedPayments {
            start_index,
            limit: Some(PER_BATCH),
        };
        let resp = wallet
            .node_client()
            .get_updated_payments(req)
            .await
            .map_err(|e| Error::Wallet(format!("get_updated_payments: {e:#}")))?;
        let batch = resp.payments;
        let batch_len = batch.len();
        if batch_len == 0 {
            break;
        }
        // Advance start_index from the *last* raw payment in this batch
        // (kept BEFORE filtering — pagination is over the full sequence).
        let last = batch.last().expect("non-empty");
        start_index = Some(PaymentUpdatedIndex {
            updated_at: last.updated_at,
            id: last.id,
        });
        scanned += batch_len;
        for p in batch.into_iter() {
            if let Some(row) = ocean_payout_from(p) {
                out.push(row);
                if out.len() >= limit_total {
                    break;
                }
            }
        }
        // If we got fewer than a full batch, we're at the tail of history.
        if batch_len < usize::from(PER_BATCH) {
            break;
        }
    }

    // Newest first; ties broken by Lexe's stable PaymentId ordering.
    out.sort_by_key(|p| std::cmp::Reverse(p.finalized_at_ms));
    Ok(out)
}

/// Single-payment filter + transform. Returns `Some(OceanPayout)` only if
/// the payment is a settled inbound BOLT12 offer payment whose payer-note
/// matches OCEAN's exact format. Factored out of [`list_offer_payouts`]
/// so the pagination loop can decide "is this row in or out?" without
/// re-doing the filter at a later stage.
fn ocean_payout_from(p: lexe_api_core::types::payments::BasicPaymentV2) -> Option<OceanPayout> {
    if p.direction != PaymentDirection::Inbound
        || p.status != PaymentStatus::Completed
        || p.kind != PaymentKind::Offer
    {
        return None;
    }
    let (block_hash, block_height) = parse_ocean_payer_note(p.message.as_deref()?)?;
    // Lightning amounts are msat-granular; OCEAN's per-block payouts are
    // commonly sub-sat (e.g. 995 msats). Round to nearest whole sat for
    // the display field; expose msat verbatim alongside it.
    let (amount_sats, amount_msat) = p
        .amount
        .as_ref()
        .map(|a| (a.round_sat().sats_u64(), a.msat()))
        .unwrap_or((0, 0));
    let finalized_at_ms = p.finalized_at.unwrap_or(p.created_at).to_i64();
    Some(OceanPayout {
        id: p.id.to_string(),
        payment_hash: p.hash.map(|h| h.to_string()),
        amount_sats,
        amount_msat,
        // `message` on the wire (lexe-api-core 0.1.14); exposed as
        // `payer_note` to match the BOLT12 spec term.
        payer_note: p.message,
        payer_name: p.payer_name,
        finalized_at_ms,
        block_hash,
        block_height,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_real_ocean_payer_note() {
        let note = "OCEAN lightning payout running at block `000000000000000000010ecb299bb3b0da3066f8e182efe8cd5390ef00094932` at height `952554`";
        let (hash, height) = parse_ocean_payer_note(note).expect("parse");
        assert_eq!(
            hash,
            "000000000000000000010ecb299bb3b0da3066f8e182efe8cd5390ef00094932"
        );
        assert_eq!(height, 952554);
    }

    #[test]
    fn rejects_non_ocean_notes() {
        assert!(parse_ocean_payer_note("").is_none());
        assert!(parse_ocean_payer_note("\t").is_none()); // user's own test payment
        assert!(parse_ocean_payer_note("hello world").is_none());
        // Missing closing backtick.
        assert!(parse_ocean_payer_note(
            "OCEAN lightning payout running at block `aaaa` at height `1`x"
        )
        .is_none());
        // Wrong hex length.
        assert!(parse_ocean_payer_note(
            "OCEAN lightning payout running at block `aaaa` at height `1`"
        )
        .is_none());
        // Non-hex char.
        let bad = format!(
            "OCEAN lightning payout running at block `{}` at height `1`",
            "z".repeat(64)
        );
        assert!(parse_ocean_payer_note(&bad).is_none());
    }

    #[test]
    fn accepts_uppercase_hex_and_normalizes() {
        let note = format!(
            "OCEAN lightning payout running at block `{}` at height `1`",
            "A".repeat(64)
        );
        let (hash, _) = parse_ocean_payer_note(&note).unwrap();
        assert!(hash
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit()));
    }
}
