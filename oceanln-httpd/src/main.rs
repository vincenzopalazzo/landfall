//! oceanln-httpd — local loopback HTTP server for the OCEAN payout flow.
//!
//! A thin transport over [`oceanln_common`] so a web app or desktop shell can
//! drive the same operations the `oceanln` CLI does, over `127.0.0.1` only.
//!
//! The BIP39 seed never crosses the HTTP boundary: the server reads it from a
//! locally configured [`SeedSource`] for each request that needs to sign or
//! touch the wallet, and never echoes it back. Seed *generation* stays a
//! human-witnessed CLI operation (`oceanln generate` / `oceanln init
//! --generate`) and is deliberately not exposed here.
//!
//! Because a browser is an intended client, the server defends the loopback
//! port: it binds loopback only, requires a bearer token on every endpoint
//! except `/health`, enforces an `Origin` allowlist (CSRF / DNS-rebinding), and
//! validates the `Host` header.

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use axum::extract::{Request, State};
use axum::http::{header, HeaderValue, Method, StatusCode};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use clap::Parser;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tower_http::cors::CorsLayer;

use oceanln_common::client::{CreateOfferReq, SidecarClient};
use oceanln_common::error::Error;
use oceanln_common::seed::SeedSource;
use oceanln_common::sign;

// ── config ──────────────────────────────────────────────────────

// No `Debug` derive: `Config` holds `--token` and `--sidecar-credentials`, and
// we don't want a stray `{:?}` to leak them (same rule as the CLI's arg structs).
#[derive(Parser)]
#[command(
    name = "oceanln-httpd",
    version,
    about = "Local loopback HTTP server exposing the OCEAN payout flow to a web/desktop frontend"
)]
struct Config {
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

struct AppState {
    seed: SeedSource,
    token: String,
    allowed_origins: Vec<String>,
    sidecar_url: String,
    sidecar_credentials: Option<String>,
    default_path: String,
}

// ── error mapping ───────────────────────────────────────────────

/// Wraps the shared error so it renders as a JSON `{ "error": "…" }` body with
/// a sensible HTTP status. Never leaks the seed — error variants carry only
/// addresses, offers, and messages.
struct ApiError(Error);

impl From<Error> for ApiError {
    fn from(e: Error) -> Self {
        ApiError(e)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = match &self.0 {
            Error::InvalidOffer(_)
            | Error::InvalidMnemonic(_)
            | Error::AddressNotP2wpkh(_)
            | Error::InvalidBip32Path(_) => StatusCode::BAD_REQUEST,
            Error::SidecarUnreachable { .. } => StatusCode::BAD_GATEWAY,
            Error::Api { code, .. } => {
                StatusCode::from_u16(*code).unwrap_or(StatusCode::BAD_GATEWAY)
            }
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };
        (status, Json(json!({ "error": self.0.to_string() }))).into_response()
    }
}

// ── handlers ────────────────────────────────────────────────────

async fn health() -> impl IntoResponse {
    Json(json!({ "status": "ok" }))
}

#[derive(Deserialize)]
struct PayoutReq {
    /// Exact OCEAN message to sign, byte-for-byte.
    message: String,
    /// Sign for an existing offer (offline) instead of creating one. When set,
    /// no sidecar call is made and the offer must be embedded in `message`.
    #[serde(default)]
    offer: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    min_amount: Option<String>,
    #[serde(default)]
    path: Option<String>,
}

#[derive(Serialize)]
struct PayoutResp {
    address: String,
    offer: String,
    message: String,
    signature: String,
}

/// End-to-end payout: resolve the offer, then derive the address and BIP-322
/// sign the message with the seed read from the configured source. Mirrors the
/// CLI's `payout` ordering — the offer is resolved before the seed is touched.
async fn payout(
    State(state): State<Arc<AppState>>,
    Json(req): Json<PayoutReq>,
) -> Result<Json<PayoutResp>, ApiError> {
    let path = sign::parse_bip32_path(req.path.as_deref().unwrap_or(&state.default_path))?;

    // 1. Resolve the offer first (the only network call, and only when creating).
    let offer = match req.offer {
        Some(offer) => {
            if !offer.starts_with("lno1") {
                return Err(Error::InvalidOffer(format!(
                    "expected a BOLT12 offer starting with 'lno1': {offer}"
                ))
                .into());
            }
            if !req.message.contains(&offer) {
                return Err(Error::InvalidOffer(
                    "offer is not present in message; OCEAN's message must embed the \
                     offer it authorizes (wrong offer or stale message?)"
                        .to_string(),
                )
                .into());
            }
            offer
        }
        None => {
            let client =
                SidecarClient::new(state.sidecar_url.clone(), state.sidecar_credentials.clone())?;
            client
                .create_offer(CreateOfferReq {
                    description: req.description.as_deref(),
                    min_amount: req.min_amount.as_deref(),
                })
                .await?
                .offer
        }
    };

    // 2. Read the seed locally, derive the key once, sign, then wipe the key.
    let secret = state.seed.load()?;
    let mnemonic = sign::parse_mnemonic(&secret)?;
    let mut key = sign::derive_private_key(&mnemonic, &path)?;
    let address = sign::address_from_key(&key)?;
    let signature = sign::sign_bip322(&key, &address, &req.message)?;
    key.inner.non_secure_erase();

    Ok(Json(PayoutResp {
        address,
        offer,
        message: req.message,
        signature,
    }))
}

#[derive(Deserialize)]
struct OfferReq {
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    min_amount: Option<String>,
}

#[derive(Serialize)]
struct OfferResp {
    offer: String,
}

/// Create a payable BOLT12 offer in-process from the configured seed.
async fn offer(
    State(state): State<Arc<AppState>>,
    Json(req): Json<OfferReq>,
) -> Result<Json<OfferResp>, ApiError> {
    let secret = state.seed.load()?;
    let offer = oceanln_common::lexe_wallet::create_offer(
        secret.as_str(),
        req.description.as_deref(),
        req.min_amount.as_deref(),
    )
    .await?;
    Ok(Json(OfferResp { offer }))
}

#[derive(Deserialize)]
struct InitReq {
    #[serde(default)]
    path: Option<String>,
}

#[derive(Serialize)]
struct InitResp {
    mining_address: String,
    provisioned: bool,
}

/// Provision the onchain Lexe wallet for the configured seed and return the
/// mining address to register with OCEAN. Idempotent. Does not generate a new
/// seed — that stays a CLI-only, human-witnessed operation.
async fn init(
    State(state): State<Arc<AppState>>,
    Json(req): Json<InitReq>,
) -> Result<Json<InitResp>, ApiError> {
    let path = sign::parse_bip32_path(req.path.as_deref().unwrap_or(&state.default_path))?;
    let secret = state.seed.load()?;
    let mnemonic = sign::parse_mnemonic(&secret)?;
    let mining_address = sign::derive_address(&mnemonic, &path)?;
    oceanln_common::lexe_wallet::init(secret.as_str()).await?;
    Ok(Json(InitResp {
        mining_address,
        provisioned: true,
    }))
}

// ── guard middleware ────────────────────────────────────────────

fn deny(status: StatusCode, msg: &str) -> Response {
    (status, Json(json!({ "error": msg }))).into_response()
}

/// Constant-time byte-slice equality, so the bearer-token comparison does not
/// leak how many leading bytes matched via early-exit timing. The length is not
/// secret (the token length is fixed), so an early length mismatch is fine.
fn ct_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

/// True if `host` (a `Host` header value, possibly `host:port`) names the
/// local machine. Anything else is rejected to blunt DNS-rebinding attacks
/// that resolve an attacker-controlled name to `127.0.0.1`.
fn host_is_loopback(host: &str) -> bool {
    // Strip an optional port. `[::1]:7762` and `127.0.0.1:7762` both supported.
    let hostname = if let Some(rest) = host.strip_prefix('[') {
        // Bracketed IPv6 literal: take up to the closing bracket.
        rest.split(']').next().unwrap_or(rest)
    } else if host.matches(':').count() > 1 {
        // Bare IPv6 (e.g. `::1`) — multiple colons, no brackets, no port.
        host
    } else {
        host.split(':').next().unwrap_or(host)
    };
    // `localhost` is not an IP literal; everything else must parse to a
    // loopback IP. A prefix test like `starts_with("127.")` would wrongly
    // accept names such as `127.evil.com` that resolve off-loopback.
    if hostname == "localhost" {
        return true;
    }
    hostname
        .parse::<std::net::IpAddr>()
        .map(|ip| ip.is_loopback())
        .unwrap_or(false)
}

/// Auth + origin + host guard applied to every route except `/health`.
async fn guard(State(state): State<Arc<AppState>>, req: Request, next: Next) -> Response {
    let headers = req.headers();

    if let Some(host) = headers.get(header::HOST).and_then(|v| v.to_str().ok()) {
        if !host_is_loopback(host) {
            return deny(StatusCode::FORBIDDEN, "host not allowed");
        }
    }

    if let Some(origin) = headers.get(header::ORIGIN).and_then(|v| v.to_str().ok()) {
        if !state.allowed_origins.iter().any(|o| o == origin) {
            return deny(StatusCode::FORBIDDEN, "origin not allowed");
        }
    }

    let expected = format!("Bearer {}", state.token);
    let authorized = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .map(|v| ct_eq(v.as_bytes(), expected.as_bytes()))
        .unwrap_or(false);
    if !authorized {
        return deny(StatusCode::UNAUTHORIZED, "missing or invalid bearer token");
    }

    next.run(req).await
}

// ── wiring ──────────────────────────────────────────────────────

fn cors_layer(allowed_origins: &[String]) -> CorsLayer {
    let origins: Vec<HeaderValue> = allowed_origins
        .iter()
        .filter_map(|o| o.parse().ok())
        .collect();
    CorsLayer::new()
        .allow_origin(origins)
        .allow_methods([Method::GET, Method::POST])
        .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE])
}

fn build_app(state: Arc<AppState>) -> Router {
    let cors = cors_layer(&state.allowed_origins);
    let protected = Router::new()
        .route("/payout", post(payout))
        .route("/offer", post(offer))
        .route("/init", post(init))
        .layer(middleware::from_fn_with_state(state.clone(), guard));

    Router::new()
        .route("/health", get(health))
        .merge(protected)
        .layer(cors)
        .with_state(state)
}

#[tokio::main]
async fn main() {
    let cfg = Config::parse();

    if !cfg.bind.ip().is_loopback() {
        eprintln!(
            "error: --bind {} is not a loopback address; this server only serves 127.0.0.1/::1",
            cfg.bind
        );
        std::process::exit(1);
    }

    let seed = SeedSource::File(cfg.seed_file.clone());
    // Fail fast: validate the seed file is present and well-formed before we
    // start accepting requests, rather than 500-ing on the first call.
    if let Err(e) = seed.load() {
        eprintln!("error: {e}");
        std::process::exit(1);
    }

    // A generated token must be revealed once so the operator can use it; an
    // operator-supplied `--token` is already known and must NOT be echoed —
    // stderr is commonly captured by systemd/Docker/supervisor logs.
    let (token, generated) = match cfg.token {
        Some(t) => (t, false),
        None => (sign::random_token(), true),
    };

    let state = Arc::new(AppState {
        seed,
        token: token.clone(),
        allowed_origins: cfg.allow_origin.clone(),
        sidecar_url: cfg.sidecar_url,
        sidecar_credentials: cfg.sidecar_credentials,
        default_path: cfg.path,
    });

    eprintln!("oceanln-httpd listening on http://{}", cfg.bind);
    if generated {
        eprintln!("bearer token (generated, shown once): {token}");
    } else {
        eprintln!("bearer token: using --token (not echoed)");
    }
    if cfg.allow_origin.is_empty() {
        eprintln!("no --allow-origin set: browser (cross-origin) clients will be rejected.");
    } else {
        eprintln!("allowed origins: {}", cfg.allow_origin.join(", "));
    }

    let app = build_app(state);
    let listener = match tokio::net::TcpListener::bind(cfg.bind).await {
        Ok(l) => l,
        Err(e) => {
            eprintln!("error: could not bind {}: {e}", cfg.bind);
            std::process::exit(1);
        }
    };
    if let Err(e) = axum::serve(listener, app).await {
        eprintln!("error: server stopped: {e}");
        std::process::exit(1);
    }
}

// ── tests ───────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use http_body_util::BodyExt;
    use tower::ServiceExt; // for `oneshot`

    const TEST_MNEMONIC: &str = "music mystery deliver gospel profit blanket leaf tell photo \
segment letter degree nice plastic duty canyon mammal marble bicycle economy unique find \
cream dune";
    const MOCK_OFFER: &str = "lno1qgsqvgnwgcg35z6ee2h3yczraddm72xrfua9uve2rlrm9deu7xyfzrcgqp0s";
    const TOKEN: &str = "test-token";

    /// Write the test seed to a uniquely-named temp file and build app state.
    fn state_with(seed_name: &str, allowed: &[&str]) -> Arc<AppState> {
        let path = std::env::temp_dir().join(format!("oceanln-httpd_{seed_name}.seed"));
        std::fs::write(&path, TEST_MNEMONIC).expect("write seed file");
        // `SeedSource::load` rejects group/world-readable seed files, so the
        // test fixture must be 0600 like a real one.
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))
                .expect("chmod 600 seed file");
        }
        Arc::new(AppState {
            seed: SeedSource::File(path),
            token: TOKEN.to_string(),
            allowed_origins: allowed.iter().map(|s| s.to_string()).collect(),
            sidecar_url: oceanln_common::client::DEFAULT_BASE_URL.to_string(),
            sidecar_credentials: None,
            default_path: sign::DEFAULT_BIP32_PATH.to_string(),
        })
    }

    async fn body_json(resp: Response) -> serde_json::Value {
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        serde_json::from_slice(&bytes).unwrap()
    }

    #[tokio::test]
    async fn health_needs_no_auth() {
        let app = build_app(state_with("health", &[]));
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn payout_without_token_is_unauthorized() {
        let app = build_app(state_with("noauth", &[]));
        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/payout")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(json!({ "message": "x" }).to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn disallowed_origin_is_forbidden() {
        let app = build_app(state_with("origin", &["http://allowed.example"]));
        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/payout")
                    .header(header::AUTHORIZATION, format!("Bearer {TOKEN}"))
                    .header(header::ORIGIN, "http://evil.example")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(json!({ "message": "x" }).to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn payout_with_offer_signs_offline() {
        let app = build_app(state_with("payout", &[]));
        let message = format!("Configure OCEAN payout to {MOCK_OFFER} at block 840000");
        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/payout")
                    .header(header::AUTHORIZATION, format!("Bearer {TOKEN}"))
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        json!({ "message": message, "offer": MOCK_OFFER }).to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let v = body_json(resp).await;
        assert_eq!(v["offer"], MOCK_OFFER);
        assert_eq!(v["message"], message);
        assert!(v["address"].as_str().unwrap().starts_with("bc1q"));
        assert!(!v["signature"].as_str().unwrap().is_empty());
        // The seed must never appear in the response.
        assert!(!body_json_contains_seed(&v));
    }

    fn body_json_contains_seed(v: &serde_json::Value) -> bool {
        v.to_string().contains("music mystery deliver")
    }

    #[tokio::test]
    async fn payout_rejects_offer_not_in_message() {
        let app = build_app(state_with("mismatch", &[]));
        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/payout")
                    .header(header::AUTHORIZATION, format!("Bearer {TOKEN}"))
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        json!({
                            "message": "Configure OCEAN payout to lno1somethingelse at block 840000",
                            "offer": MOCK_OFFER
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn host_is_loopback_accepts_loopback_only() {
        // Loopback names/IPs, with and without a port.
        for ok in [
            "localhost",
            "localhost:7762",
            "127.0.0.1",
            "127.0.0.1:7762",
            "127.0.0.2", // all of 127.0.0.0/8 is loopback
            "::1",
            "[::1]:7762",
        ] {
            assert!(host_is_loopback(ok), "{ok} should be loopback");
        }
        // The bug this test guards: a name that merely *starts with* "127."
        // must not pass, nor any routable host.
        for bad in [
            "127.evil.com",
            "127.0.0.1.evil.com",
            "evil.com",
            "10.0.0.1",
            "0.0.0.0",
            "192.168.1.1",
        ] {
            assert!(!host_is_loopback(bad), "{bad} must not be loopback");
        }
    }

    #[test]
    fn ct_eq_matches_only_identical_slices() {
        assert!(ct_eq(b"Bearer abc", b"Bearer abc"));
        assert!(!ct_eq(b"Bearer abc", b"Bearer abd"));
        assert!(!ct_eq(b"short", b"longer value"));
        assert!(ct_eq(b"", b""));
    }
}
