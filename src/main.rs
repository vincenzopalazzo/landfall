//! oceanln — OCEAN Lightning payout CLI.

use clap::Parser;
use oceanln::cli::{Cli, Command, ConfigureArgs, InvoiceArgs, PayArgs, PaymentArgs, SignArgs};
use oceanln::client::{CreateInvoiceReq, PayInvoiceReq, SidecarClient};
use oceanln::error::Result;
use oceanln::{ocean, sign};
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
        Command::Sign(args) => cmd_sign(args, cli.json),
        Command::Health => {
            let client = SidecarClient::new(cli.url, cli.credentials)?;
            cmd_health(&client, cli.json).await
        }
        Command::Info => {
            let client = SidecarClient::new(cli.url, cli.credentials)?;
            cmd_info(&client, cli.json).await
        }
        Command::Configure(args) => {
            let client = SidecarClient::new(cli.url, cli.credentials)?;
            cmd_configure(&client, args, cli.json).await
        }
        Command::Invoice(args) => {
            let client = SidecarClient::new(cli.url, cli.credentials)?;
            cmd_invoice(&client, args, cli.json).await
        }
        Command::Pay(args) => {
            let client = SidecarClient::new(cli.url, cli.credentials)?;
            cmd_pay(&client, args, cli.json).await
        }
        Command::Payment(args) => {
            let client = SidecarClient::new(cli.url, cli.credentials)?;
            cmd_payment(&client, args, cli.json).await
        }
    }
}

// ── sign ────────────────────────────────────────────────────────

#[derive(Serialize)]
struct SignOutput<'a> {
    message: &'a str,
    address: &'a str,
    path: String,
    signature: String,
}

fn cmd_sign(args: SignArgs, json: bool) -> Result<()> {
    let path = sign::parse_bip32_path(&args.path)?;
    let secret = sign::prompt_mnemonic()?;
    let mnemonic = sign::parse_mnemonic(&secret)?;

    let signature = sign::sign_bip322(&mnemonic, &args.address, &path, &args.message)?;

    if json {
        let out = SignOutput {
            message: &args.message,
            address: &args.address,
            path: args.path.clone(),
            signature,
        };
        print_json(&out)
    } else {
        println!("Signing OCEAN message via BIP-322...");
        println!("Message: {}", args.message);
        println!("Address: {}", args.address);
        println!();
        println!("BIP-322 Signature:");
        println!("{signature}");
        println!();
        println!("Paste this signature into the OCEAN web interface.");
        Ok(())
    }
}

// ── health ──────────────────────────────────────────────────────

async fn cmd_health(client: &SidecarClient, json: bool) -> Result<()> {
    let h = client.health().await?;
    if json {
        print_json(&h)
    } else {
        println!("Status: {}", h.status);
        Ok(())
    }
}

// ── info ────────────────────────────────────────────────────────

async fn cmd_info(client: &SidecarClient, json: bool) -> Result<()> {
    let i = client.node_info().await?;
    if json {
        print_json(&i)
    } else {
        println!("Version:             {}", i.version);
        println!("Node PK:             {}", i.node_pk);
        println!("User PK:             {}", i.user_pk);
        println!("Balance:             {} sats", i.balance);
        println!("  Lightning:         {} sats", i.lightning_balance);
        println!("  Sendable:          {} sats", i.lightning_sendable_balance);
        println!("  On-chain:          {} sats", i.onchain_balance);
        println!(
            "Channels:            {} ({} usable)",
            i.num_channels, i.num_usable_channels
        );
        Ok(())
    }
}

// ── configure ───────────────────────────────────────────────────

#[derive(Serialize)]
struct ConfigureOutput<'a> {
    message: &'a str,
    offer: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    address: Option<&'a str>,
}

async fn cmd_configure(_client: &SidecarClient, args: ConfigureArgs, json: bool) -> Result<()> {
    let offer_string = args.offer;
    ocean::validate_bolt12_offer(&offer_string)?;

    if json {
        print_json(&ConfigureOutput {
            message: &args.message,
            offer: &offer_string,
            address: args.address.as_deref(),
        })
    } else {
        println!("OCEAN Lightning Payout Configuration");
        println!("====================================\n");
        println!("BOLT12 Offer:  {}", offer_string);
        if let Some(a) = &args.address {
            println!("BTC Address:   {}", a);
        }
        println!("\nMessage to sign:");
        println!("  {}", args.message);
        println!("\nSign this message with the private key of your OCEAN mining");
        println!("address using BIP-322. Paste the base64 signature into the");
        println!("OCEAN web interface.\n");
        println!("To sign with this CLI:");
        println!(
            "  oceanln sign --message {:?} --address {}",
            args.message,
            args.address.as_deref().unwrap_or("<bc1q...>")
        );
        Ok(())
    }
}

// ── invoice ─────────────────────────────────────────────────────

async fn cmd_invoice(client: &SidecarClient, args: InvoiceArgs, json: bool) -> Result<()> {
    let inv = client
        .create_invoice(CreateInvoiceReq {
            amount: Some(&args.amount),
            description: args.description.as_deref(),
            expiration_secs: Some(3600),
            payer_note: None,
        })
        .await?;
    if json {
        print_json(&inv)
    } else {
        println!("Invoice:       {}", inv.invoice);
        println!(
            "Amount:        {} sats",
            inv.amount.as_deref().unwrap_or("amountless")
        );
        println!("Payment hash:  {}", inv.payment_hash);
        println!("Expires at:    {}", inv.expires_at);
        Ok(())
    }
}

// ── pay ─────────────────────────────────────────────────────────

async fn cmd_pay(client: &SidecarClient, args: PayArgs, json: bool) -> Result<()> {
    let resp = client
        .pay_invoice(PayInvoiceReq {
            invoice: &args.bolt11,
            fallback_amount: None,
            note: None,
            payer_note: None,
        })
        .await?;
    if json {
        print_json(&resp)
    } else {
        println!("Payment sent.");
        println!("Index:         {}", resp.index);
        println!("Created at:    {}", resp.created_at);
        Ok(())
    }
}

// ── payment ─────────────────────────────────────────────────────

async fn cmd_payment(client: &SidecarClient, args: PaymentArgs, json: bool) -> Result<()> {
    let p = match client.payment(&args.index).await? {
        Some(p) => p,
        None => {
            println!("Payment not found.");
            return Ok(());
        }
    };
    if json {
        print_json(&p)
    } else {
        println!("Index:         {}", p.index);
        println!("Rail:          {}", p.rail);
        println!("Kind:          {}", p.kind);
        println!("Direction:     {}", p.direction);
        println!("Status:        {}", p.status);
        println!("Status msg:    {}", p.status_msg);
        if let Some(amt) = &p.amount {
            println!("Amount:        {} sats", amt);
        }
        println!("Fees:          {} sats", p.fees);
        if let Some(inv) = &p.invoice {
            println!("Invoice:       {}", inv);
        }
        if let Some(addr) = &p.address {
            println!("Address:       {}", addr);
        }
        if let Some(note) = &p.note {
            println!("Note:          {}", note);
        }
        println!("Created at:    {}", p.created_at);
        println!("Updated at:    {}", p.updated_at);
        if let Some(ts) = p.finalized_at {
            println!("Finalized at:  {}", ts);
        }
        Ok(())
    }
}

// ── helpers ─────────────────────────────────────────────────────

fn print_json<T: Serialize>(v: &T) -> Result<()> {
    let s = serde_json::to_string_pretty(v)?;
    println!("{s}");
    Ok(())
}
