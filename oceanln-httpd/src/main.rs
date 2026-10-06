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
    /// Mutually exclusive with `--no-auth`.
    #[arg(long, conflicts_with = "no_auth")]
    token: Option<String>,

    /// Disable bearer authentication entirely. Intended for v1 single-host
    /// deployments where the server binds loopback only and there is no
    /// other process on the machine that should be trusted differently
    /// from the operator. Origin + Host header guards still apply. Adding
    /// auth later is a one-line `--token` swap on relaunch.
    #[arg(long, default_value_t = false)]
    no_auth: bool,

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

    /// QA ONLY (feature `qa-mock`): replace the Lexe wallet with an in-memory
    /// stub so `/init`, `/offer`, `/node`, … answer without a node. Seed
    /// handling and BIP-322 signing stay real. Absent from release builds.
    #[cfg(feature = "qa-mock")]
    #[arg(long, default_value_t = false)]
    mock_wallet: bool,
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

    // Token resolution:
    //   --no-auth        → empty token (the `guard` middleware treats
    //                      empty as "skip bearer check"). Loopback bind
    //                      + Origin/Host guards remain.
    //   --token X        → bearer required, fixed value
    //   neither          → bearer required, value auto-generated and
    //                      printed once to stderr so the operator can
    //                      copy it.
    let (token, mode) = match (cli.no_auth, cli.token) {
        (true, _) => (String::new(), TokenMode::Disabled),
        (false, Some(t)) => (t, TokenMode::Supplied),
        (false, None) => (sign::random_token(), TokenMode::Generated),
    };

    eprintln!("oceanln-httpd listening on http://{}", cli.bind);
    match mode {
        TokenMode::Generated => {
            eprintln!("bearer token (generated, shown once): {token}");
        }
        TokenMode::Supplied => {
            eprintln!("bearer token: using --token (not echoed)");
        }
        TokenMode::Disabled => {
            eprintln!(
                "AUTH DISABLED (--no-auth): bearer check is OFF. Loopback bind + Origin/Host \
                 guards are the only defenses. Do NOT expose this listener to the network."
            );
        }
    }
    if cli.allow_origin.is_empty() {
        eprintln!("no --allow-origin set: browser (cross-origin) clients will be rejected.");
    } else {
        eprintln!("allowed origins: {}", cli.allow_origin.join(", "));
    }

    let wallet: Arc<dyn oceanln_httpd::WalletProvider> = {
        #[cfg(feature = "qa-mock")]
        if cli.mock_wallet {
            eprintln!(
                "MOCK WALLET (--mock-wallet): Lexe is NOT contacted; offers/balances are stubs. QA only."
            );
            Arc::new(oceanln_httpd::qa_mock::MockWalletProvider)
        } else {
            Arc::new(LexeWalletProvider)
        }
        #[cfg(not(feature = "qa-mock"))]
        Arc::new(LexeWalletProvider)
    };

    let state = Arc::new(AppState::new(
        ServerConfig {
            seed,
            token,
            allowed_origins: cli.allow_origin,
            sidecar_url: cli.sidecar_url,
            sidecar_credentials: cli.sidecar_credentials,
            default_path: cli.path,
        },
        wallet,
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

/// Pretty-print discriminator for the startup banner; not used elsewhere.
enum TokenMode {
    /// User explicitly opted into unauthed loopback (v1 single-host deploy).
    Disabled,
    /// Operator provided `--token`.
    Supplied,
    /// Auto-generated; printed once so the operator can copy it.
    Generated,
}
