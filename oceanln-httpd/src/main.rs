//! oceanln-httpd binary — parse CLI flags, wire up the production
//! [`LexeWalletProvider`], and serve the app from [`oceanln_httpd`] on a
//! loopback socket. All request handling lives in the library crate.

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use clap::Parser;

use oceanln_common::seed::SeedSource;
use oceanln_common::sign;
use oceanln_httpd::{build_app, AppState, LexeWalletProvider, ServerConfig};

// No `Debug` derive: `Cli` holds `--token` and `--sidecar-credentials`, and we
// don't want a stray `{:?}` to leak them (same rule as the CLI's arg structs).
#[derive(Parser)]
#[command(
    name = "oceanln-httpd",
    version,
    about = "Local loopback HTTP server exposing the OCEAN payout flow to a web/desktop frontend"
)]
struct Cli {
    /// Loopback address to bind. Must be a loopback IP (127.0.0.0/8 or ::1) —
    /// the server refuses to expose the seed-backed API on a routable address.
    #[arg(long, default_value = "127.0.0.1:7762")]
    bind: SocketAddr,

    /// Path to the file holding the 24-word mnemonic. Read per request; the
    /// seed is never accepted over HTTP nor returned in a response.
    #[arg(long)]
    seed_file: PathBuf,

    /// Bearer token required on every endpoint except `/health`. If omitted, a
    /// fresh 256-bit token is generated and printed to stderr on startup.
    #[arg(long)]
    token: Option<String>,

    /// Allowed `Origin` for browser clients (repeatable). Requests carrying an
    /// `Origin` not on this list are rejected (403). Native clients that send
    /// no `Origin` are unaffected. Empty list = no cross-origin browser access.
    #[arg(long = "allow-origin")]
    allow_origin: Vec<String>,

    /// Default BIP32 derivation path for signing/derivation (per-request
    /// `path` overrides it).
    #[arg(long, default_value = sign::DEFAULT_BIP32_PATH)]
    path: String,

    /// Lexe sidecar URL used when `/payout` must create an offer (no `offer`
    /// in the request body). Kept server-side so a client cannot redirect the
    /// call to an arbitrary host (SSRF).
    #[arg(long, default_value = oceanln_common::client::DEFAULT_BASE_URL)]
    sidecar_url: String,

    /// Bearer credentials for the sidecar. Server-side only; never taken from
    /// a request body.
    #[arg(long)]
    sidecar_credentials: Option<String>,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    if !cli.bind.ip().is_loopback() {
        eprintln!(
            "error: --bind {} is not a loopback address; this server only serves 127.0.0.1/::1",
            cli.bind
        );
        std::process::exit(1);
    }

    let seed = SeedSource::File(cli.seed_file.clone());
    // If a seed file is already present, validate it now (fail fast on a
    // malformed or insecure-permission file rather than 500-ing on first use).
    // If it is absent, that's fine — the wizard's `/generate` or `/import`
    // creates it; only the seed-touching endpoints error until then.
    if cli.seed_file.exists() {
        if let Err(e) = seed.load() {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
    } else {
        eprintln!(
            "note: seed file {} does not exist yet — POST /generate or /import to create a wallet.",
            cli.seed_file.display()
        );
    }

    // A generated token must be revealed once so the operator can use it; an
    // operator-supplied `--token` is already known and must NOT be echoed —
    // stderr is commonly captured by systemd/Docker/supervisor logs.
    let (token, generated) = match cli.token {
        Some(t) => (t, false),
        None => (sign::random_token(), true),
    };

    eprintln!("oceanln-httpd listening on http://{}", cli.bind);
    if generated {
        eprintln!("bearer token (generated, shown once): {token}");
    } else {
        eprintln!("bearer token: using --token (not echoed)");
    }
    if cli.allow_origin.is_empty() {
        eprintln!("no --allow-origin set: browser (cross-origin) clients will be rejected.");
    } else {
        eprintln!("allowed origins: {}", cli.allow_origin.join(", "));
    }

    let state = Arc::new(AppState::new(
        ServerConfig {
            seed,
            token,
            allowed_origins: cli.allow_origin,
            sidecar_url: cli.sidecar_url,
            sidecar_credentials: cli.sidecar_credentials,
            default_path: cli.path,
        },
        Arc::new(LexeWalletProvider),
    ));

    let app = build_app(state);
    let listener = match tokio::net::TcpListener::bind(cli.bind).await {
        Ok(l) => l,
        Err(e) => {
            eprintln!("error: could not bind {}: {e}", cli.bind);
            std::process::exit(1);
        }
    };
    if let Err(e) = axum::serve(listener, app).await {
        eprintln!("error: server stopped: {e}");
        std::process::exit(1);
    }
}
