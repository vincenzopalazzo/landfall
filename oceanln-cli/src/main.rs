//! oceanln — OCEAN Lightning payout CLI.

mod cli;

use clap::Parser;
use cli::{Cli, Command, PayoutArgs};
#[cfg(feature = "lexe-sdk")]
use cli::{InitArgs, OfferArgs};
use oceanln_common::client::{CreateOfferReq, SidecarClient};
use oceanln_common::error::{Error, Result};
use oceanln_common::sign;
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
        Command::Payout(args) => cmd_payout(args, cli.json).await,
        #[cfg(feature = "lexe-sdk")]
        Command::Init(args) => cmd_init(args, cli.json).await,
        #[cfg(feature = "lexe-sdk")]
        Command::Offer(args) => cmd_offer(args, cli.json).await,
    }
}

// ── init / offer (in-process Lexe SDK, feature `lexe-sdk`) ───────

#[cfg(feature = "lexe-sdk")]
#[derive(Serialize)]
struct InitOutput<'a> {
    /// Only present when the seed was freshly generated (`--generate`).
    #[serde(skip_serializing_if = "Option::is_none")]
    mnemonic: Option<&'a str>,
    /// The BIP84 address to register with OCEAN as your mining payout address.
    mining_address: &'a str,
    /// False under `--dry-run` (seed + address derived, wallet not provisioned).
    provisioned: bool,
    /// Where the seed was persisted, if at all. Absent under `--no-store` /
    /// `--dry-run`; future `offer` / `payout` read it without re-prompting.
    #[serde(skip_serializing_if = "Option::is_none")]
    seed_file: Option<String>,
}

/// One-shot onboarding: get a seed (generated or provided), derive the mining
/// address, and (unless `--dry-run`) provision the onchain Lexe wallet.
#[cfg(feature = "lexe-sdk")]
async fn cmd_init(args: InitArgs, json: bool) -> Result<()> {
    let path = sign::parse_bip32_path(&args.path)?;

    // Seed: fresh (--generate) or resolved (seed file / stdin / managed file /
    // prompt).
    let secret = if args.generate {
        sign::generate_mnemonic()?
    } else {
        sign::resolve_seed(args.seed_file.as_deref())?
    };
    let mnemonic = sign::parse_mnemonic(&secret)?;

    // The mining address to register with OCEAN — derived locally, instantly.
    let mining_address = sign::derive_address(&mnemonic, &path)?;

    // Surface a freshly generated seed BEFORE any fallible step (persistence or
    // the provisioning network call), so neither a store conflict nor a network
    // failure can ever lose words the user has not seen yet.
    if args.generate {
        if json {
            // In JSON mode the seed is only in the final object (printed after
            // provisioning) — echo to stderr now so a later failure can't lose it.
            if !args.dry_run {
                eprintln!(
                    "WARNING: write these 24 words down (echoed here in case a later step fails):"
                );
                eprintln!("{}", secret.as_str());
            }
        } else {
            eprintln!("WARNING: write these 24 words down — they ARE your wallet.");
            println!("Seed:            {}", secret.as_str());
        }
    }

    // Persist the seed (now that it has been surfaced) so future `offer` /
    // `payout` runs derive keys without re-prompting. Skipped under --dry-run
    // (pure preview) and --no-store.
    let stored = if args.dry_run || args.no_store {
        None
    } else {
        Some(sign::store_seed(
            &secret,
            args.seed_file.as_deref(),
            args.force,
        )?)
    };

    if !json {
        println!("Mining address:  {mining_address}");
        println!("  ^ register this address with OCEAN as your payout address.");
        if let Some(path) = &stored {
            println!("Seed saved to:   {} (0600)", path.display());
        }
    }

    if args.dry_run {
        if !json {
            eprintln!("(dry run) wallet NOT provisioned — no network call made.");
        }
    } else {
        if !json {
            eprintln!("Provisioning your Lexe wallet (this contacts Lexe)...");
        }
        oceanln_common::lexe_wallet::init(secret.as_str()).await?;
        if !json {
            println!("Lexe wallet provisioned.");
        }
    }

    if json {
        print_json(&InitOutput {
            mnemonic: args.generate.then(|| secret.as_str()),
            mining_address: &mining_address,
            provisioned: !args.dry_run,
            seed_file: stored.map(|p| p.display().to_string()),
        })
    } else {
        Ok(())
    }
}

#[cfg(feature = "lexe-sdk")]
#[derive(Serialize)]
struct OfferOutput<'a> {
    offer: &'a str,
}

#[cfg(feature = "lexe-sdk")]
async fn cmd_offer(args: OfferArgs, json: bool) -> Result<()> {
    let secret = sign::resolve_seed(args.seed_file.as_deref())?;
    sign::parse_mnemonic(&secret)?;
    let offer = oceanln_common::lexe_wallet::create_offer(
        secret.as_str(),
        args.description.as_deref(),
        args.min_amount.as_deref(),
    )
    .await?;
    if json {
        print_json(&OfferOutput { offer: &offer })
    } else {
        println!("BOLT12 offer:\n{offer}");
        println!("\nRegister this offer with OCEAN, then sign the message it gives you:");
        println!("  oceanln payout --offer {offer} --message '<OCEAN message>'");
        Ok(())
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
    #[cfg(feature = "lexe-sdk")]
    {
        eprintln!(
            "Next: `oceanln init --generate` does seed + wallet + mining address in one step,"
        );
        eprintln!("or feed this seed to `oceanln offer` / `oceanln payout`.");
    }
    #[cfg(not(feature = "lexe-sdk"))]
    {
        eprintln!("Use the same mnemonic for both the OCEAN signing and the Lexe node:");
        eprintln!("  LEXE_ROOT_SEED_PATH=<file containing these words> lexe-sidecar");
    }
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
async fn cmd_payout(args: PayoutArgs, json: bool) -> Result<()> {
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
            // OCEAN's message embeds the offer being configured. If --offer
            // isn't present in --message, the signature would authorize a
            // different offer than the one we print — reject the mismatch
            // (stale clipboard / wrong --offer).
            if !args.message.contains(&offer) {
                return Err(Error::InvalidOffer(
                    "--offer is not present in --message; OCEAN's message must \
                     embed the offer it authorizes (wrong --offer or stale message?)"
                        .to_string(),
                ));
            }
            offer
        }
        None => {
            let client = SidecarClient::new(args.url, args.credentials)?;
            client
                .create_offer(CreateOfferReq {
                    description: args.description.as_deref(),
                    min_amount: args.min_amount.as_deref(),
                })
                .await?
                .offer
        }
    };

    // 2. Now the secret: resolve (seed file / stdin / managed file / prompt),
    //    validate, and derive the signing key ONCE (BIP39 PBKDF2 is expensive).
    //    The mining address the user registers with OCEAN comes from the same
    //    key, so the key provably controls it.
    let secret = sign::resolve_seed(args.seed_file.as_deref())?;
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
