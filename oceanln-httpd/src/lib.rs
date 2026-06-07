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
use serde::Deserialize;
use serde_json::json;
use tower_http::cors::CorsLayer;

use oceanln_common::error::{Error, Result};
use oceanln_common::seed::SeedSource;

pub mod service;

// ── wallet seam ─────────────────────────────────────────────────

/// The wallet operations the server exposes, abstracted so the HTTP layer does
/// not depend directly on the in-process Lexe SDK. The real implementation is
/// [`LexeWalletProvider`]; tests substitute their own implementation.
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

    /// List the wallet's inbound, completed BOLT12 offer payments — i.e.
    /// OCEAN's Lightning payouts. Empty list = no payouts yet (the Lexe
    /// node call may still have succeeded). `limit` caps the round-trip.
    async fn list_offer_payouts(
        &self,
        mnemonic: &str,
        limit: u16,
    ) -> Result<Vec<oceanln_common::lexe_wallet::OceanPayout>>;
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

    async fn list_offer_payouts(
        &self,
        mnemonic: &str,
        limit: u16,
    ) -> Result<Vec<oceanln_common::lexe_wallet::OceanPayout>> {
        oceanln_common::lexe_wallet::list_offer_payouts(mnemonic, limit).await
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
        // Single source of truth for Error → status, shared with the desktop
        // transport via `service::http_status`.
        let status = StatusCode::from_u16(service::http_status(&self.0))
            .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
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

/// Thin adapter over [`service::payout`].
async fn payout(
    State(state): State<Arc<AppState>>,
    Json(req): Json<PayoutReq>,
) -> std::result::Result<Json<service::PayoutResp>, ApiError> {
    let resp = service::payout(
        &state.cfg.seed,
        &state.cfg.default_path,
        &state.cfg.sidecar_url,
        state.cfg.sidecar_credentials.as_deref(),
        req.message,
        req.offer,
        req.description.as_deref(),
        req.min_amount.as_deref(),
        req.path.as_deref(),
    )
    .await?;
    Ok(Json(resp))
}

#[derive(Deserialize)]
struct OfferReq {
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    min_amount: Option<String>,
}

/// Thin adapter over [`service::create_offer`].
async fn offer(
    State(state): State<Arc<AppState>>,
    Json(req): Json<OfferReq>,
) -> std::result::Result<Json<service::OfferResp>, ApiError> {
    let resp = service::create_offer(
        &state.cfg.seed,
        state.wallet.as_ref(),
        req.description.as_deref(),
        req.min_amount.as_deref(),
    )
    .await?;
    Ok(Json(resp))
}

#[derive(Deserialize)]
struct InitReq {
    #[serde(default)]
    path: Option<String>,
}

/// Thin adapter over [`service::init`].
async fn init(
    State(state): State<Arc<AppState>>,
    Json(req): Json<InitReq>,
) -> std::result::Result<Json<service::InitResp>, ApiError> {
    let resp = service::init(
        &state.cfg.seed,
        &state.cfg.default_path,
        state.wallet.as_ref(),
        req.path.as_deref(),
    )
    .await?;
    Ok(Json(resp))
}

/// Offline wallet status (does a wallet exist + its address/offer) so a client
/// can skip onboarding on launch. Touches the seed but makes no network call.
async fn status(
    State(state): State<Arc<AppState>>,
) -> std::result::Result<Json<service::StatusResp>, ApiError> {
    let resp = service::status(&state.cfg.seed, &state.cfg.default_path)?;
    Ok(Json(resp))
}

/// Thin adapter over [`service::generate`]. The "revealed once" semantics, the
/// no-`force` rule, and the atomic create all live in the service layer; this
/// just logs the operator-facing note and serializes.
async fn generate(
    State(state): State<Arc<AppState>>,
) -> std::result::Result<Json<service::GenerateResp>, ApiError> {
    let resp = service::generate(&state.cfg.seed, &state.cfg.default_path)?;
    eprintln!("oceanln-httpd: generated a new recovery phrase (revealed once via /generate)");
    Ok(Json(resp))
}

#[derive(Deserialize)]
struct ImportReq {
    /// The 24-word recovery phrase to import.
    mnemonic: String,
    #[serde(default)]
    force: bool,
}

/// Thin adapter over [`service::import`]. The phrase crosses the wire once, by
/// design; the service wipes the buffer after taking a zeroizing copy.
async fn import(
    State(state): State<Arc<AppState>>,
    Json(mut req): Json<ImportReq>,
) -> std::result::Result<Json<service::ImportResp>, ApiError> {
    let resp = service::import(
        &state.cfg.seed,
        &state.cfg.default_path,
        &mut req.mnemonic,
        req.force,
    )?;
    Ok(Json(resp))
}

#[derive(Deserialize, Default)]
struct PayoutsQuery {
    /// Cap on how many payouts to return. Defaults to 100. Hard cap is
    /// enforced inside `oceanln_common::lexe_wallet::list_offer_payouts`
    /// (paginated against the Lexe node's `MAX_PAYMENTS_BATCH_SIZE`).
    #[serde(default)]
    limit: Option<u16>,
}

/// Thin adapter over [`service::list_offer_payouts`]. Returns the same
/// `Vec<OceanPayout>` the CLI and Tauri IPC return — single source of truth.
async fn payouts(
    State(state): State<Arc<AppState>>,
    axum::extract::Query(q): axum::extract::Query<PayoutsQuery>,
) -> std::result::Result<Json<Vec<oceanln_common::lexe_wallet::OceanPayout>>, ApiError> {
    let resp = service::list_offer_payouts(
        &state.cfg.seed,
        state.wallet.as_ref(),
        q.limit.unwrap_or(100),
    )
    .await?;
    Ok(Json(resp))
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

/// Build the router for `state`: `/health` is open; `/status`, `/generate`,
/// `/import`, `/payout`, `/offer`, and `/init` sit behind the [`guard`] (token +
/// origin + host) and the CORS layer.
pub fn build_app(state: Arc<AppState>) -> Router {
    let cors = cors_layer(&state.cfg.allowed_origins);
    let protected = Router::new()
        .route("/status", get(status))
        .route("/generate", post(generate))
        .route("/import", post(import))
        .route("/payout", post(payout))
        .route("/offer", post(offer))
        .route("/init", post(init))
        .route("/payouts", get(payouts))
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
