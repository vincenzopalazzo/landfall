//! oceanln — OCEAN Lightning payout CLI.

use clap::Parser;
use oceanln::cli::{Cli, Command, PayoutArgs};
use oceanln::client::{CreateOfferReq, SidecarClient};
use oceanln::error::{Error, Result};
use oceanln::sign;
use serde::Serialize;

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    if let Err(e) = run(cli).await {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

async fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Command::Generate => cmd_generate(cli.json),
        Command::Payout(args) => cmd_payout(cli.url, cli.credentials, args, cli.json).await,
    }
}

// ── generate ────────────────────────────────────────────────────

#[derive(Serialize)]
struct GenerateOutput<'a> {
    mnemonic: &'a str,
}

/// Generate a fresh 24-word seed and print it exactly once.
///
/// The mnemonic is written to stdout (so `--json` / piping stays clean);
/// the warning and usage hint go to stderr so they never contaminate the
/// captured seed.
fn cmd_generate(json: bool) -> Result<()> {
    let secret = sign::generate_mnemonic()?;

    if json {
        return print_json(&GenerateOutput {
            mnemonic: secret.as_str(),
        });
    }

    eprintln!("WARNING: write these 24 words down somewhere safe. They ARE your wallet —");
    eprintln!("this is the only time they are shown, and they are never written to disk.");
    eprintln!();
    println!("{}", secret.as_str());
    eprintln!();
    eprintln!("Use the same mnemonic for both the OCEAN signing and the Lexe node:");
    eprintln!("  oceanln payout --message '<OCEAN message>' --description '<offer description>'");
    eprintln!("  LEXE_ROOT_SEED_PATH=<file containing these words> lexe-sidecar");
    Ok(())
}

// ── payout ──────────────────────────────────────────────────────

#[derive(Serialize)]
struct PayoutOutput<'a> {
    address: &'a str,
    offer: &'a str,
    message: &'a str,
    signature: String,
}

/// End-to-end OCEAN payout setup. Order matters: resolve the offer (the only
/// network call, and only when creating) *before* prompting for the mnemonic,
/// so a sidecar failure aborts the flow without ever asking the user for — or
/// touching — their seed.
async fn cmd_payout(
    url: String,
    credentials: Option<String>,
    args: PayoutArgs,
    json: bool,
) -> Result<()> {
    let path = sign::parse_bip32_path(&args.path)?;

    // 1. Resolve the offer. With --offer the user signs for an existing offer
    //    (offer-first OCEAN flow) and no sidecar is contacted at all. Otherwise
    //    the node creates a fresh payable offer — done before the mnemonic
    //    prompt so a sidecar failure aborts before the secret is entered.
    let offer = match args.offer {
        Some(offer) => {
            if !offer.starts_with("lno1") {
                return Err(Error::InvalidOffer(format!(
                    "expected a BOLT12 offer starting with 'lno1': {offer}"
                )));
            }
            offer
        }
        None => {
            let client = SidecarClient::new(url, credentials)?;
            client
                .create_offer(CreateOfferReq {
                    description: args.description.as_deref(),
                    min_amount: args.min_amount.as_deref(),
                })
                .await?
                .offer
        }
    };

    // 2. Now the secret: prompt, validate, and derive the signing key ONCE
    //    (BIP39 PBKDF2 is expensive). The mining address the user registers
    //    with OCEAN comes from the same key, so the key provably controls it.
    let secret = sign::prompt_mnemonic()?;
    let mnemonic = sign::parse_mnemonic(&secret)?;
    let mut key = sign::derive_private_key(&mnemonic, &path)?;
    let address = sign::address_from_key(&key)?;

    // 3. BIP-322 sign the exact OCEAN message with the derived key, then wipe
    //    it — secp256k1 0.29 does not zeroize `SecretKey` on drop.
    let signature = sign::sign_bip322(&key, &address, &args.message)?;
    key.inner.non_secure_erase();

    if json {
        print_json(&PayoutOutput {
            address: &address,
            offer: &offer,
            message: &args.message,
            signature,
        })
    } else {
        println!("OCEAN Payout Setup");
        println!("==================\n");
        println!("Mining address:  {address}");
        println!("BOLT12 offer:    {offer}");
        println!("Message:         {}", args.message);
        println!();
        println!("BIP-322 signature:");
        println!("{signature}");
        println!();
        println!("Register the mining address and offer with OCEAN, then paste the");
        println!("signature into the OCEAN web interface to authorize the payout.");
        Ok(())
    }
}

// ── helpers ─────────────────────────────────────────────────────

fn print_json<T: Serialize>(v: &T) -> Result<()> {
    let s = serde_json::to_string_pretty(v)?;
    println!("{s}");
    Ok(())
}
