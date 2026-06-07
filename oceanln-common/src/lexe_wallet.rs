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
use lexe::types::command::{CreateInvoiceRequest, CreateOfferRequest, PayRequest};
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

/// Live node status + balances, read from the in-process Lexe node.
/// Powers the dashboard's "Node wallet" cards (Lightning channel /
/// On-chain) and the "Node online" chip. All sat amounts are rounded to
/// whole sats for display; the wire values are exact node figures (no
/// mock, no scrape).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeStatus {
    /// The node's public key (hex, the `node_id`) — node identity.
    pub node_pk: String,
    /// Total channels (usable or not).
    pub num_channels: usize,
    /// Channels currently usable for sending (peer online, channel ready).
    pub num_usable_channels: usize,
    /// Total Lightning balance across all channels, in sats.
    pub lightning_total_sats: u64,
    /// Conservative upper bound on what we can send over LN right now, in
    /// sats (channel reserve / pending HTLCs / fees accounted for).
    pub lightning_sendable_sats: u64,
    /// Total on-chain balance, including unconfirmed, in sats.
    pub onchain_total_sats: u64,
    /// Trusted on-chain balance: confirmed + own unconfirmed, in sats.
    pub onchain_trusted_sats: u64,
    /// Sum of Lightning + on-chain balance, in sats.
    pub total_balance_sats: u64,
}

/// Read live balances + channel counts from the node.
pub async fn node_status(mnemonic: &str) -> Result<NodeStatus> {
    let seed = root_seed(mnemonic)?;
    let wallet = wallet(&seed)?;
    let info = wallet
        .node_info()
        .await
        .map_err(|e| Error::Wallet(format!("node info: {e:#}")))?;
    Ok(NodeStatus {
        node_pk: info.node_pk.to_string(),
        num_channels: info.num_channels,
        num_usable_channels: info.num_usable_channels,
        lightning_total_sats: info.lightning_balance.round_sat().sats_u64(),
        lightning_sendable_sats: info.lightning_sendable_balance.round_sat().sats_u64(),
        onchain_total_sats: info.onchain_balance.round_sat().sats_u64(),
        onchain_trusted_sats: info.onchain_trusted_balance.round_sat().sats_u64(),
        total_balance_sats: info.balance.round_sat().sats_u64(),
    })
}

/// One row of the node's full payment activity — inbound *and* outbound,
/// Lightning *and* on-chain — for the dashboard's "Node activity" panel.
/// Unlike [`OceanPayout`] (OCEAN payouts only), this surfaces *every*
/// payment, flagging the OCEAN ones via [`Self::is_ocean`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Activity {
    /// Lexe `PaymentId` (`<kind>_<hex>`), unique per payment.
    pub id: String,
    /// `"in"` (inbound) or `"out"` (outbound).
    pub direction: String,
    /// `"ln"` (Lightning) or `"onchain"`.
    pub rail: String,
    /// Net sats, rounded to nearest whole sat. `0` if amount unset.
    pub amount_sats: u64,
    /// Net msats — exact wire amount, no rounding.
    pub amount_msat: u64,
    /// `"settled"`, `"pending"`, or `"failed"`.
    pub status: String,
    /// BOLT12 payer note / on-chain label, if any.
    pub note: Option<String>,
    /// Payer's self-reported name, if any.
    pub counterparty: Option<String>,
    /// Finalized (or created) time, epoch milliseconds.
    pub finalized_at_ms: i64,
    /// Lightning payment hash (hex), if applicable.
    pub payment_hash: Option<String>,
    /// On-chain txid (hex), if applicable.
    pub txid: Option<String>,
    /// True iff this is a verified OCEAN payout (inbound BOLT12 offer
    /// payment whose payer note matches OCEAN's exact signature).
    pub is_ocean: bool,
    /// Block height parsed from an OCEAN payer note (OCEAN rows only).
    pub block_height: Option<u64>,
}

/// List the node's full payment history (newest first), mapped to
/// [`Activity`] rows. Same pagination + scan-cap discipline as
/// [`list_offer_payouts`], but WITHOUT the OCEAN-only filter — every
/// payment is returned, with OCEAN ones flagged.
pub async fn list_payments(mnemonic: &str, limit: u16) -> Result<Vec<Activity>> {
    let seed = root_seed(mnemonic)?;
    let wallet = wallet(&seed)?;
    const PER_BATCH: u16 = 100;
    let mut start_index: Option<PaymentUpdatedIndex> = None;
    let mut out: Vec<Activity> = Vec::new();
    let limit_total = usize::from(limit);
    let mut scanned: usize = 0;

    loop {
        if out.len() >= limit_total || scanned >= MAX_PAYMENTS_SCANNED {
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
        let last = batch.last().expect("non-empty");
        start_index = Some(PaymentUpdatedIndex {
            updated_at: last.updated_at,
            id: last.id,
        });
        scanned += batch_len;
        for p in batch.into_iter() {
            out.push(activity_from(p));
            if out.len() >= limit_total {
                break;
            }
        }
        if batch_len < usize::from(PER_BATCH) {
            break;
        }
    }

    out.sort_by_key(|a| std::cmp::Reverse(a.finalized_at_ms));
    Ok(out)
}

/// Map a raw Lexe payment to an [`Activity`] row (no filtering — every
/// payment maps to a row).
fn activity_from(p: lexe_api_core::types::payments::BasicPaymentV2) -> Activity {
    let inbound = p.direction == PaymentDirection::Inbound;
    let rail = if p.kind == PaymentKind::Onchain {
        "onchain"
    } else {
        "ln"
    };
    let status = match p.status {
        PaymentStatus::Completed => "settled",
        PaymentStatus::Failed => "failed",
        _ => "pending",
    };
    let (amount_sats, amount_msat) = p
        .amount
        .as_ref()
        .map(|a| (a.round_sat().sats_u64(), a.msat()))
        .unwrap_or((0, 0));
    let ocean = inbound
        && p.kind == PaymentKind::Offer
        && p.message
            .as_deref()
            .and_then(parse_ocean_payer_note)
            .is_some();
    let block_height = p
        .message
        .as_deref()
        .and_then(parse_ocean_payer_note)
        .map(|(_, h)| h);
    let finalized_at_ms = p.finalized_at.unwrap_or(p.created_at).to_i64();
    Activity {
        id: p.id.to_string(),
        direction: (if inbound { "in" } else { "out" }).to_string(),
        rail: rail.to_string(),
        amount_sats,
        amount_msat,
        status: status.to_string(),
        note: p.message.clone(),
        counterparty: p.payer_name.clone(),
        finalized_at_ms,
        payment_hash: p.hash.map(|h| h.to_string()),
        // On-chain payments carry their txid in the payment hash slot for
        // our purposes; if Lexe exposes it separately in future, map it
        // here. For now LN rows have a hash, on-chain rows may not.
        txid: None,
        is_ocean: ocean,
        block_height,
    }
}

/// Summary of an outbound payment we just sent (Send flow).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaySummary {
    /// Lexe `PaymentId` of the outbound payment.
    pub id: String,
    /// Amount sent, in sats (echoed from the request; `0` if the payable
    /// carried its own amount and none was supplied).
    pub amount_sats: u64,
    /// When we attempted the payment, epoch milliseconds.
    pub created_at_ms: i64,
}

/// Create a BOLT11 invoice on the node to RECEIVE a payment (Receive flow).
/// `amount_sats = None` mints an amountless invoice (payer chooses).
pub async fn create_invoice(
    mnemonic: &str,
    amount_sats: Option<u64>,
    description: Option<&str>,
) -> Result<String> {
    let seed = root_seed(mnemonic)?;
    let wallet = wallet(&seed)?;
    let amount = sats_to_amount(amount_sats)?;
    let resp = wallet
        .create_invoice(CreateInvoiceRequest {
            amount,
            description: description.map(String::from),
            ..Default::default()
        })
        .await
        .map_err(|e| Error::Wallet(format!("create invoice: {e:#}")))?;
    Ok(resp.invoice.to_string())
}

/// Send a payment to any payable string — BOLT11 invoice, BOLT12 offer,
/// Lightning address, LNURL, or on-chain address — via the Lexe wallet's
/// universal `pay`. **Moves real funds; irreversible.** `amount_sats` is
/// required for amountless payables and ignored when the payable already
/// carries an amount.
pub async fn pay(
    mnemonic: &str,
    payable: &str,
    amount_sats: Option<u64>,
    note: Option<&str>,
) -> Result<PaySummary> {
    let seed = root_seed(mnemonic)?;
    let wallet = wallet(&seed)?;
    let amount = sats_to_amount(amount_sats)?;
    let resp = wallet
        .pay(PayRequest {
            payable: payable.trim().to_string(),
            amount,
            message: note.map(String::from),
            personal_note: None,
        })
        .await
        .map_err(|e| Error::Wallet(format!("pay: {e:#}")))?;
    Ok(PaySummary {
        id: resp.index.id.to_string(),
        amount_sats: amount_sats.unwrap_or(0),
        created_at_ms: resp.created_at.to_i64(),
    })
}

/// Convert an optional whole-sat amount into a Lexe [`Amount`]. The Lexe
/// `Amount::from_str` parses satoshis (same as `--min-amount`).
fn sats_to_amount(sats: Option<u64>) -> Result<Option<Amount>> {
    match sats {
        Some(s) => {
            Ok(Some(Amount::from_str(&s.to_string()).map_err(|e| {
                Error::Wallet(format!("invalid amount: {e}"))
            })?))
        }
        None => Ok(None),
    }
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
