//! CLI argument definitions (clap).

use crate::client::DEFAULT_BASE_URL;
use crate::sign::DEFAULT_BIP32_PATH;
use clap::{Args, Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(
    name = "oceanln",
    version,
    about = "OCEAN Lightning payout CLI — BIP-322 message signing + Lexe sidecar client"
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
    /// Sign a message via BIP-322 using a BIP39 mnemonic prompted on stdin.
    Sign(SignArgs),

    /// Show Lexe node info (balance, channels, keys).
    Info,

    /// Build the OCEAN payout configuration message + offer summary.
    Configure(ConfigureArgs),

    /// Create a BOLT11 invoice.
    Invoice(InvoiceArgs),

    /// Pay a BOLT11 invoice.
    Pay(PayArgs),

    /// Look up a payment by index.
    Payment(PaymentArgs),

    /// Lexe sidecar health check.
    Health,
}

#[derive(Args, Debug)]
pub struct SignArgs {
    /// The exact message text from the OCEAN web interface.
    #[arg(long)]
    pub message: String,

    /// Bitcoin address (P2WPKH bc1q…) registered with OCEAN.
    #[arg(long)]
    pub address: String,

    /// BIP32 derivation path (default: m/84'/0'/0'/0/0).
    #[arg(long, default_value = DEFAULT_BIP32_PATH)]
    pub path: String,
}

#[derive(Args, Debug)]
pub struct ConfigureArgs {
    /// Bitcoin address registered with OCEAN (context only — not signed).
    #[arg(long)]
    pub address: Option<String>,

    /// The OCEAN configuration message text (copy from the web UI).
    #[arg(long)]
    pub message: String,

    /// BOLT12 offer override. If omitted, fetched from the Lexe node.
    #[arg(long)]
    pub offer: Option<String>,
}

#[derive(Args, Debug)]
pub struct InvoiceArgs {
    /// Amount in satoshis.
    pub amount: String,
    /// Optional invoice description.
    pub description: Option<String>,
}

#[derive(Args, Debug)]
pub struct PayArgs {
    /// BOLT11 invoice string.
    pub bolt11: String,
}

#[derive(Args, Debug)]
pub struct PaymentArgs {
    /// Payment index returned by `pay` or `invoice`.
    pub index: String,
}
