//! CLI argument definitions (clap).

use clap::{Args, Parser, Subcommand};
use oceanln_common::client::DEFAULT_BASE_URL;
use oceanln_common::sign::DEFAULT_BIP32_PATH;
use std::path::PathBuf;

// No `Debug` derive: `PayoutArgs` holds `--credentials`, and we don't want a
// stray `{:?}` to leak it. (The other arg structs hold no secrets.)
#[derive(Parser)]
#[command(
    name = "oceanln",
    version,
    about = "OCEAN Lightning payout CLI — create a Lexe wallet + BOLT12 offer and BIP-322 sign for OCEAN",
    after_help = AFTER_HELP,
)]
pub struct Cli {
    /// Output as machine-readable JSON.
    #[arg(long, global = true)]
    pub json: bool,

    #[command(subcommand)]
    pub command: Command,
}

#[cfg(feature = "lexe-sdk")]
const AFTER_HELP: &str = "Examples (in-process, no sidecar):\n  \
    oceanln init --generate                         # new wallet + mining address, one shot\n  \
    oceanln offer --description 'OCEAN payout'      # create a BOLT12 offer\n  \
    oceanln payout --offer lno1... --message '<exact OCEAN message>'   # BIP-322 sign\n\n\
    `init --dry-run` previews the seed + mining address without provisioning.";

#[cfg(not(feature = "lexe-sdk"))]
const AFTER_HELP: &str = "Examples (thin build — needs a running lexe-sidecar):\n  \
    oceanln generate\n  \
    oceanln payout --message '<OCEAN message>' --description 'my pool payout'\n  \
    oceanln payout --offer lno1... --message '<OCEAN message>'   # offline sign\n\n\
    Build with the in-process Lexe wallet (default): `cargo build` adds `init`/`offer`.";

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Generate a fresh 24-word BIP39 mnemonic seed.
    Generate,

    /// End-to-end OCEAN payout: derive the mining address from the mnemonic,
    /// obtain a payable BOLT12 offer, and BIP-322 sign the OCEAN message.
    Payout(PayoutArgs),

    /// Onboard in one shot (in-process, no sidecar): generate or take a seed,
    /// provision the onchain Lexe wallet, and print the mining address to
    /// register with OCEAN. Run once before `offer`.
    #[cfg(feature = "lexe-sdk")]
    Init(InitArgs),

    /// Create a payable BOLT12 offer in-process (no sidecar) and print it.
    #[cfg(feature = "lexe-sdk")]
    Offer(OfferArgs),
}

#[cfg(feature = "lexe-sdk")]
#[derive(Args, Debug)]
pub struct InitArgs {
    /// Generate a fresh 24-word seed instead of reading one from stdin.
    /// Use this to create a brand-new wallet in one shot.
    #[arg(long)]
    pub generate: bool,

    /// Derive and print the seed + mining address WITHOUT provisioning the
    /// wallet (no network call). Useful for previewing or testing.
    #[arg(long)]
    pub dry_run: bool,

    /// BIP32 derivation path for the mining address (default: m/84'/0'/0'/0/0).
    #[arg(long, default_value = DEFAULT_BIP32_PATH)]
    pub path: String,

    /// Read/write the seed at this path instead of the default
    /// (`$XDG_CONFIG_HOME/oceanln/seed`, else `~/.config/oceanln/seed`).
    #[arg(long)]
    pub seed_file: Option<PathBuf>,

    /// Overwrite an existing seed file that holds a different seed.
    #[arg(long)]
    pub force: bool,

    /// Do not persist the seed to disk (one-off provisioning; you'll be
    /// prompted again on the next `offer` / `payout`).
    #[arg(long)]
    pub no_store: bool,
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

    /// Read the seed from this path instead of the default managed file
    /// (`~/.config/oceanln/seed`). Falls back to a prompt if no seed is found.
    #[arg(long)]
    pub seed_file: Option<PathBuf>,
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

    /// Description baked into the BOLT12 offer the sidecar creates.
    #[arg(long)]
    pub description: Option<String>,

    /// Optional minimum amount for the offer, in satoshis. Omit for a
    /// variable-amount offer the payer chooses.
    #[arg(long)]
    pub min_amount: Option<String>,

    /// BIP32 derivation path for the mining address (default: m/84'/0'/0'/0/0).
    #[arg(long, default_value = DEFAULT_BIP32_PATH)]
    pub path: String,

    /// Lexe sidecar URL (only used when creating an offer, i.e. without --offer).
    #[arg(long, default_value = DEFAULT_BASE_URL)]
    pub url: String,

    /// Bearer credentials for the sidecar (only used without --offer).
    #[arg(long)]
    pub credentials: Option<String>,

    /// Read the seed from this path instead of the default managed file
    /// (`~/.config/oceanln/seed`). Falls back to a prompt if no seed is found.
    #[arg(long)]
    pub seed_file: Option<PathBuf>,
}
