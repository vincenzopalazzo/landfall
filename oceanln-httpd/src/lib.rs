//! oceanln-httpd — local loopback HTTP server for the OCEAN payout flow.
//!
//! A thin transport over [`oceanln_common`] so a web app or desktop shell can
//! drive the same operations the `oceanln` CLI does, over `127.0.0.1` only.
//!
//! For the signing/wallet endpoints the BIP39 seed never crosses the HTTP
//! boundary: the server reads it from a locally configured [`SeedSource`] per
//! request and never echoes it back. The `/generate` and `/import` endpoints
//! are the deliberate exceptions — they create/accept the phrase for the
//! onboarding wizard and persist it to the seed file. `/generate` reveals the
//! new phrase exactly once and refuses to clobber an existing wallet; both stay
//! gated by the loopback bind + bearer token + Origin allowlist.
//!
//! Because a browser is an intended client, the server defends the loopback
//! port: it binds loopback only, requires a bearer token on every endpoint
//! except `/health`, enforces an `Origin` allowlist (CSRF / DNS-rebinding), and
//! validates the `Host` header.
//!
//! The wallet-touching endpoints (`/offer`, `/init`) go through the
//! [`WalletProvider`] trait so the transport is decoupled from the in-process
//! Lexe SDK and can be exercised in tests with a mock.

use std::sync::Arc;

use axum::extract::{Request, State};
use axum::http::{header, HeaderValue, Method, StatusCode};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tower_http::cors::CorsLayer;
use zeroize::Zeroize;

use oceanln_common::client::{CreateOfferReq, SidecarClient};
use oceanln_common::error::{Error, Result};
use oceanln_common::seed::SeedSource;
use oceanln_common::sign::{self, MnemonicSecret};

// ── wallet seam ─────────────────────────────────────────────────

/// The wallet operations the server exposes, abstracted so the HTTP layer does
/// not depend directly on the in-process Lexe SDK. The real implementation is
/// [`LexeWalletProvider`]; tests substitute a mock.
#[async_trait::async_trait]
pub trait WalletProvider: Send + Sync {
    /// Provision the onchain wallet for `mnemonic` (idempotent).
    async fn provision(&self, mnemonic: &str) -> Result<()>;

    /// Create a payable BOLT12 offer for `mnemonic`; returns the `lno1…` string.
    async fn create_offer(
        &self,
        mnemonic: &str,
        description: Option<&str>,
        min_amount: Option<&str>,
    ) -> Result<String>;
}

/// Production [`WalletProvider`] backed by the in-process Lexe SDK
/// ([`oceanln_common::lexe_wallet`]).
pub struct LexeWalletProvider;

#[async_trait::async_trait]
impl WalletProvider for LexeWalletProvider {
    async fn provision(&self, mnemonic: &str) -> Result<()> {
        oceanln_common::lexe_wallet::init(mnemonic).await
    }

    async fn create_offer(
        &self,
        mnemonic: &str,
        description: Option<&str>,
        min_amount: Option<&str>,
    ) -> Result<String> {
        oceanln_common::lexe_wallet::create_offer(mnemonic, description, min_amount).await
    }
}

// ── configuration / state ───────────────────────────────────────

/// Runtime configuration for the server, resolved from CLI flags (or built
/// directly in tests). Holds secrets (`token`, `sidecar_credentials`), so it
/// deliberately does not derive `Debug`.
pub struct ServerConfig {
    /// Where the mnemonic is read from, per request. Never crosses the wire.
    pub seed: SeedSource,
    /// Bearer token required on every endpoint except `/health`.
    pub token: String,
    /// Exact `Origin` values browser clients may use; empty = no cross-origin.
    pub allowed_origins: Vec<String>,
    /// Lexe sidecar URL for the `/payout` create-offer path. Server-side only.
    pub sidecar_url: String,
    /// Bearer credentials for the sidecar. Server-side only.
    pub sidecar_credentials: Option<String>,
    /// Default BIP32 derivation path; a per-request `path` overrides it.
    pub default_path: String,
}

/// Shared application state handed to every request.
pub struct AppState {
    cfg: ServerConfig,
    wallet: Arc<dyn WalletProvider>,
}

impl AppState {
    pub fn new(cfg: ServerConfig, wallet: Arc<dyn WalletProvider>) -> Self {
        Self { cfg, wallet }
    }
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
            Error::SeedExists { .. } => StatusCode::CONFLICT,
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
) -> std::result::Result<Json<PayoutResp>, ApiError> {
    let path = sign::parse_bip32_path(req.path.as_deref().unwrap_or(&state.cfg.default_path))?;

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
            let client = SidecarClient::new(
                state.cfg.sidecar_url.clone(),
                state.cfg.sidecar_credentials.clone(),
            )?;
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
    let secret = state.cfg.seed.load()?;
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
) -> std::result::Result<Json<OfferResp>, ApiError> {
    let secret = state.cfg.seed.load()?;
    let offer = state
        .wallet
        .create_offer(
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

/// Provision the onchain wallet for the configured seed and return the mining
/// address to register with OCEAN. Idempotent. Operates on the already-stored
/// seed — use `/generate` or `/import` first to create one.
async fn init(
    State(state): State<Arc<AppState>>,
    Json(req): Json<InitReq>,
) -> std::result::Result<Json<InitResp>, ApiError> {
    let path = sign::parse_bip32_path(req.path.as_deref().unwrap_or(&state.cfg.default_path))?;
    let secret = state.cfg.seed.load()?;
    let mnemonic = sign::parse_mnemonic(&secret)?;
    let mining_address = sign::derive_address(&mnemonic, &path)?;
    state.wallet.provision(secret.as_str()).await?;
    Ok(Json(InitResp {
        mining_address,
        provisioned: true,
    }))
}

#[derive(Serialize)]
struct GenerateResp {
    /// The freshly generated 24-word phrase — revealed exactly once so the user
    /// can back it up. (Persisted server-side; not re-readable afterwards.)
    mnemonic: String,
    mining_address: String,
}

/// Generate a fresh 24-word recovery phrase, persist it to the configured
/// seed file, derive the mining address, and reveal the phrase once.
///
/// Refuses with 409 if a seed file already exists — there is deliberately no
/// `force`: generating a new phrase over an existing wallet would irreversibly
/// destroy it, so replacing a wallet must go through `/import` (an explicit,
/// user-supplied phrase). Checking existence before generating also means a
/// refusal never strands a revealed phrase. This relaxes the "seed never
/// crosses the wire" rule for the generate step; it stays gated by the loopback
/// bind + bearer token + Origin allowlist.
async fn generate(
    State(state): State<Arc<AppState>>,
) -> std::result::Result<Json<GenerateResp>, ApiError> {
    let dest = state.cfg.seed.path();
    // Never overwrite an existing wallet from /generate; fail before generating
    // so a refusal never strands a revealed phrase.
    if dest.exists() {
        return Err(Error::SeedExists {
            path: dest.display().to_string(),
        }
        .into());
    }
    let path = sign::parse_bip32_path(&state.cfg.default_path)?;
    let secret = sign::generate_mnemonic()?;
    let mnemonic = sign::parse_mnemonic(&secret)?;
    let mining_address = sign::derive_address(&mnemonic, &path)?;
    sign::store_seed(&secret, Some(dest), false)?;
    eprintln!("oceanln-httpd: generated a new recovery phrase (revealed once via /generate)");
    Ok(Json(GenerateResp {
        mnemonic: secret.as_str().to_string(),
        mining_address,
    }))
}

#[derive(Deserialize)]
struct ImportReq {
    /// The 24-word recovery phrase to import.
    mnemonic: String,
    #[serde(default)]
    force: bool,
}

#[derive(Serialize)]
struct ImportResp {
    mining_address: String,
}

/// Import an existing 24-word phrase: validate it, persist it to the configured
/// seed file (409 if one already exists unless `force`), and return the derived
/// mining address. The phrase crosses the wire once, by design.
async fn import(
    State(state): State<Arc<AppState>>,
    Json(mut req): Json<ImportReq>,
) -> std::result::Result<Json<ImportResp>, ApiError> {
    let path = sign::parse_bip32_path(&state.cfg.default_path)?;
    let secret = MnemonicSecret::from_input(&req.mnemonic);
    // We hold a zeroizing copy now; wipe the request-body phrase. (The transport
    // buffers upstream are out of our control, but don't keep a second copy.)
    req.mnemonic.zeroize();
    let mnemonic = sign::parse_mnemonic(&secret)?; // 400 on a non-24-word phrase
    let mining_address = sign::derive_address(&mnemonic, &path)?;
    sign::store_seed(&secret, Some(state.cfg.seed.path()), req.force)?;
    Ok(Json(ImportResp { mining_address }))
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
        if !state.cfg.allowed_origins.iter().any(|o| o == origin) {
            return deny(StatusCode::FORBIDDEN, "origin not allowed");
        }
    }

    let expected = format!("Bearer {}", state.cfg.token);
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

/// Build the router for `state`: `/health` is open; `/generate`, `/import`,
/// `/payout`, `/offer`, and `/init` sit behind the [`guard`] (token + origin +
/// host) and the CORS layer.
pub fn build_app(state: Arc<AppState>) -> Router {
    let cors = cors_layer(&state.cfg.allowed_origins);
    let protected = Router::new()
        .route("/generate", post(generate))
        .route("/import", post(import))
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

// ── unit tests for the guard predicates ─────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

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
