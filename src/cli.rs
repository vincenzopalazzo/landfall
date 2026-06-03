//! CLI argument definitions (clap).

use crate::client::DEFAULT_BASE_URL;
use crate::sign::DEFAULT_BIP32_PATH;
use clap::{Args, Parser, Subcommand};

// No `Debug` derive: `Cli` holds `--credentials`, and we don't want a stray
// `{:?}` to leak it. The subcommand arg structs (no secrets) keep `Debug`.
#[derive(Parser)]
#[command(
    name = "oceanln",
    version,
    about = "OCEAN Lightning payout CLI — BIP-322 signing + BOLT12 offer over a Lexe sidecar",
    after_help = "Examples:\n  \
        oceanln generate\n  \
        oceanln payout --message 'Configure OCEAN payout to lno1... at block 840000' --description 'my pool payout'\n  \
        oceanln payout --message '...' --description '...' --min-amount 1000 --credentials $LEXE_CLIENT_CREDENTIALS\n\n\
        The sidecar must be running separately:\n  \
        lexe-sidecar --client-credentials-path <path>"
)]
pub struct Cli {
    /// Lexe sidecar URL.
    #[arg(long, global = true, default_value = DEFAULT_BASE_URL)]
    pub url: String,

    /// Bearer credentials for sidecar authentication.
    #[arg(long, global = true)]
    pub credentials: Option<String>,

    /// Output as machine-readable JSON.
    #[arg(long, global = true)]
    pub json: bool,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Generate a fresh 24-word BIP39 mnemonic seed.
    Generate,

    /// End-to-end OCEAN payout setup: derive the mining address from the
    /// mnemonic, create a payable BOLT12 offer on the node, and BIP-322
    /// sign the OCEAN message — printing address, offer, and signature.
    Payout(PayoutArgs),

    /// Create + provision the onchain Lexe wallet from the mnemonic (in-process,
    /// no sidecar). Run once before `offer`.
    #[cfg(feature = "lexe-sdk")]
    Init,

    /// Create a payable BOLT12 offer in-process (no sidecar) and print it.
    #[cfg(feature = "lexe-sdk")]
    Offer(OfferArgs),
}

#[cfg(feature = "lexe-sdk")]
#[derive(Args, Debug)]
pub struct OfferArgs {
    /// Description encoded into the offer (shown to payers).
    #[arg(long)]
    pub description: Option<String>,

    /// Minimum offer amount in satoshis. Omit for a variable-amount offer.
    #[arg(long)]
    pub min_amount: Option<String>,
}

#[derive(Args, Debug)]
pub struct PayoutArgs {
    /// The exact OCEAN configuration message (copy verbatim from the web UI).
    /// Signed byte-for-byte — do not edit it.
    #[arg(long)]
    pub message: String,

    /// Sign for an EXISTING BOLT12 offer instead of creating one. When set,
    /// no sidecar call is made — payout just derives the address and signs
    /// (fully offline). This is the offer-first OCEAN flow: create/register
    /// the offer, get the message from OCEAN, then sign it here.
    /// Mutually exclusive with --description / --min-amount.
    #[arg(long, conflicts_with_all = ["description", "min_amount"])]
    pub offer: Option<String>,

    /// Description baked into the BOLT12 offer the node creates.
    #[arg(long)]
    pub description: Option<String>,

    /// Optional minimum amount for the offer, in satoshis. Omit for a
    /// variable-amount offer the payer chooses.
    #[arg(long)]
    pub min_amount: Option<String>,

    /// BIP32 derivation path for the mining address (default: m/84'/0'/0'/0/0).
    #[arg(long, default_value = DEFAULT_BIP32_PATH)]
    pub path: String,
}
