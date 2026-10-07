//! landfall — CLI for OCEAN Lightning payouts.

mod cli;

use clap::Parser;
use cli::{Cli, Command, PayoutArgs, VerifyArgs};
#[cfg(feature = "lexe-sdk")]
use cli::{InitArgs, OfferArgs, PayoutsArgs};
use landfall_common::client::{CreateOfferReq, SidecarClient};
use landfall_common::error::{Error, Result};
use landfall_common::sign;
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
        Command::Verify(args) => cmd_verify(args, cli.json),
        #[cfg(feature = "lexe-sdk")]
        Command::Init(args) => cmd_init(args, cli.json).await,
        #[cfg(feature = "lexe-sdk")]
        Command::Offer(args) => cmd_offer(args, cli.json).await,
        #[cfg(feature = "lexe-sdk")]
        Command::Payouts(args) => cmd_payouts(args, cli.json).await,
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
        landfall_common::lexe_wallet::init(secret.as_str()).await?;
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
    let offer = landfall_common::lexe_wallet::create_offer(
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
        println!("  landfall payout --offer {offer} --message '<OCEAN message>'");
        Ok(())
    }
}

// ── payouts ─────────────────────────────────────────────────────

/// List OCEAN's Lightning payouts to this wallet's BOLT12 offer.
///
/// Reads through the SAME `landfall_common::lexe_wallet::list_offer_payouts`
/// function the HTTP `GET /payouts` route and the Tauri desktop IPC use —
/// the filter (OCEAN payer-note matching, status/direction/kind gates,
/// pagination against the node's `MAX_PAYMENTS_BATCH_SIZE`) lives in one
/// place. The CLI just formats the result.
#[cfg(feature = "lexe-sdk")]
async fn cmd_payouts(args: PayoutsArgs, json: bool) -> Result<()> {
    let secret = sign::resolve_seed(args.seed_file.as_deref())?;
    sign::parse_mnemonic(&secret)?;
    let rows =
        landfall_common::lexe_wallet::list_offer_payouts(secret.as_str(), args.limit).await?;

    if json {
        return print_json(&rows);
    }
    if rows.is_empty() {
        eprintln!(
            "No OCEAN payouts yet. They show up here once your unpaid balance clears OCEAN's"
        );
        eprintln!("minimum Lightning payout threshold.");
        return Ok(());
    }
    println!(
        "{:<19}  {:>14}  {:>9}  Payment hash",
        "Settled (UTC)", "Amount (sats)", "Block",
    );
    println!("{}", "─".repeat(19 + 16 + 11 + 64));
    for r in &rows {
        let when = chrono_like_iso(r.finalized_at_ms);
        let hash = r.payment_hash.as_deref().unwrap_or("—");
        println!(
            "{:<19}  {:>14}  {:>9}  {}",
            when, r.amount_sats, r.block_height, hash
        );
    }
    Ok(())
}

/// Format an epoch-milliseconds value as `YYYY-MM-DD HH:MM` UTC without
/// pulling in `chrono` for one call site. Uses Rust's std-library
/// breakdown of a Unix timestamp.
#[cfg(feature = "lexe-sdk")]
fn chrono_like_iso(ms: i64) -> String {
    use std::time::{Duration, UNIX_EPOCH};
    if ms <= 0 {
        return "—".to_string();
    }
    let secs = (ms / 1000) as u64;
    let dt = UNIX_EPOCH + Duration::from_secs(secs);
    let total_secs = dt.duration_since(UNIX_EPOCH).unwrap().as_secs();
    let days = total_secs / 86_400;
    let secs_in_day = total_secs % 86_400;
    let hour = secs_in_day / 3600;
    let minute = (secs_in_day % 3600) / 60;
    let (year, month, day) = days_to_ymd(days as i64);
    format!("{year:04}-{month:02}-{day:02} {hour:02}:{minute:02}")
}

/// Days since 1970-01-01 → (year, month, day) in the proleptic Gregorian
/// calendar. Algorithm from Howard Hinnant's "date" library — small,
/// dependency-free, correct for the full range we care about.
#[cfg(feature = "lexe-sdk")]
fn days_to_ymd(days: i64) -> (i32, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = (y + if m <= 2 { 1 } else { 0 }) as i32;
    (year, m as u32, d as u32)
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
            "Next: `landfall init --generate` does seed + wallet + mining address in one step,"
        );
        eprintln!("or feed this seed to `landfall offer` / `landfall payout`.");
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

// ── verify ──────────────────────────────────────────────────────

#[derive(Serialize)]
struct VerifyOutput<'a> {
    valid: bool,
    address: &'a str,
    message: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    reason: Option<String>,
}

/// Offline BIP-322 check — what OCEAN does with the three values you paste.
/// No seed is read and nothing is contacted, so this is safe to run anywhere
/// (a QA box, a CI job, a second machine) to confirm a signature before
/// submitting it. Exit status 1 on an invalid signature so scripts can gate
/// on it; `--json` adds a `reason` field on failure.
fn cmd_verify(args: VerifyArgs, json: bool) -> Result<()> {
    let outcome = sign::verify_bip322(&args.address, &args.message, &args.signature);
    let (valid, reason) = match &outcome {
        Ok(()) => (true, None),
        Err(e) => (false, Some(e.to_string())),
    };
    if json {
        print_json(&VerifyOutput {
            valid,
            address: &args.address,
            message: &args.message,
            reason,
        })?;
    } else if valid {
        println!(
            "valid: BIP-322 signature by the key controlling {}",
            args.address
        );
    } else {
        println!(
            "INVALID: {}",
            reason.as_deref().unwrap_or("signature does not verify")
        );
    }
    outcome
}

// ── helpers ─────────────────────────────────────────────────────

fn print_json<T: Serialize>(v: &T) -> Result<()> {
    let s = serde_json::to_string_pretty(v)?;
    println!("{s}");
    Ok(())
}

#[cfg(all(test, feature = "lexe-sdk"))]
mod tests {
    use super::*;

    #[test]
    fn date_helper_known_vectors() {
        // 0 ms → epoch
        assert_eq!(chrono_like_iso(0), "—");
        // 1970-01-01 00:00:01 (1000 ms)
        assert_eq!(chrono_like_iso(1_000), "1970-01-01 00:00");
        // The live OCEAN payout we observed (`fr_51d1f72e…`): 1780736697678 ms
        // should render as "2026-06-06 09:04" UTC.
        assert_eq!(chrono_like_iso(1_780_736_697_678), "2026-06-06 09:04");
        // 2000-01-01 00:00:00 UTC (946684800000 ms)
        assert_eq!(chrono_like_iso(946_684_800_000), "2000-01-01 00:00");
    }

    #[test]
    fn date_helper_handles_leap_year() {
        // 2024-02-29 12:34:56 UTC = 1709210096000 ms (a real leap-day).
        assert_eq!(chrono_like_iso(1_709_210_096_000), "2024-02-29 12:34");
    }

    #[test]
    fn date_helper_handles_year_boundary() {
        // 1999-12-31 23:59:00 UTC = 946684740000 ms.
        assert_eq!(chrono_like_iso(946_684_740_000), "1999-12-31 23:59");
        // 2000-03-01 00:00:00 UTC (post-leap) = 951868800000 ms.
        assert_eq!(chrono_like_iso(951_868_800_000), "2000-03-01 00:00");
    }
}
