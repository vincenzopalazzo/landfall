//! End-to-end tests that drive a real bound socket over HTTP.
//!
//! Unlike a `tower::oneshot` test, these spawn the server with `axum::serve`
//! on an ephemeral loopback port and hit it with `reqwest`, so the full hyper
//! stack runs — including the CORS preflight (`OPTIONS`) path, which short-
//! circuits ahead of the auth guard. The wallet-touching endpoints (`/offer`,
//! `/init`) run against a `MockWallet`, so no Lexe backend is required.

use std::sync::Arc;

use oceanln_common::error::Result;
use oceanln_common::seed::SeedSource;
use oceanln_common::sign::DEFAULT_BIP32_PATH;
use oceanln_httpd::{build_app, AppState, ServerConfig, WalletProvider};

/// 24-word seed whose BIP84 address is `bc1qpstw48j7j9gjugw25jmjvd96jlwgdnedk5pr6r`.
const TEST_MNEMONIC: &str = "music mystery deliver gospel profit blanket leaf tell photo \
segment letter degree nice plastic duty canyon mammal marble bicycle economy unique find \
cream dune";
const MOCK_OFFER: &str = "lno1qgsqvgnwgcg35z6ee2h3yczraddm72xrfua9uve2rlrm9deu7xyfzrcgqp0s";
const TOKEN: &str = "integration-token";
const MOCK_PROVISIONED_OFFER: &str = "lno1mockprovideroffer";
/// A different valid 24-word phrase, for the forced-seed-replacement test.
const OTHER_MNEMONIC: &str = "abandon abandon abandon abandon abandon abandon abandon abandon \
abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon \
abandon abandon abandon art";

/// A `WalletProvider` that never touches the network, so `/offer` and `/init`
/// can be tested without a Lexe backend.
struct MockWallet;

#[async_trait::async_trait]
impl WalletProvider for MockWallet {
    async fn provision(&self, _mnemonic: &str) -> Result<()> {
        Ok(())
    }
    async fn create_offer(
        &self,
        _mnemonic: &str,
        _description: Option<&str>,
        _min_amount: Option<&str>,
    ) -> Result<String> {
        Ok(MOCK_PROVISIONED_OFFER.to_string())
    }
    async fn list_offer_payouts(
        &self,
        _mnemonic: &str,
        _limit: u16,
    ) -> Result<Vec<oceanln_common::lexe_wallet::OceanPayout>> {
        Ok(vec![])
    }
}

/// A uniquely-named temp seed-file path for a test.
fn seed_path(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("oceanln-httpd-it-{name}.seed"))
}

/// Write the test seed to a uniquely-named 0600 temp file.
fn write_seed(name: &str) -> std::path::PathBuf {
    let path = seed_path(name);
    std::fs::write(&path, TEST_MNEMONIC).expect("write seed file");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).expect("chmod 600");
    }
    path
}

/// Spawn the server for a given seed source on an ephemeral loopback port.
async fn spawn_with(seed: SeedSource, allowed_origins: &[&str]) -> String {
    let state = Arc::new(AppState::new(
        ServerConfig {
            seed,
            token: TOKEN.to_string(),
            allowed_origins: allowed_origins.iter().map(|s| s.to_string()).collect(),
            sidecar_url: oceanln_common::client::DEFAULT_BASE_URL.to_string(),
            sidecar_credentials: None,
            default_path: DEFAULT_BIP32_PATH.to_string(),
        },
        Arc::new(MockWallet),
    ));
    let app = build_app(state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind ephemeral port");
    let addr = listener.local_addr().expect("local addr");
    tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });
    format!("http://{addr}")
}

/// Spawn with a pre-existing 0600 seed file.
async fn spawn(name: &str, allowed_origins: &[&str]) -> String {
    spawn_with(SeedSource::File(write_seed(name)), allowed_origins).await
}

/// Spawn with a seed-file path that does NOT yet exist — for `/generate` and
/// `/import`, which create it. Returns the base URL and the seed path.
async fn spawn_no_seed(name: &str, allowed_origins: &[&str]) -> (String, std::path::PathBuf) {
    let path = seed_path(name);
    let _ = std::fs::remove_file(&path); // ensure a clean slate across re-runs
    let url = spawn_with(SeedSource::File(path.clone()), allowed_origins).await;
    (url, path)
}

fn client() -> reqwest::Client {
    reqwest::Client::new()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn health_needs_no_auth() {
    let base = spawn("health", &[]).await;
    let resp = client().get(format!("{base}/health")).send().await.unwrap();
    assert_eq!(resp.status(), 200);
    assert_eq!(
        resp.json::<serde_json::Value>().await.unwrap()["status"],
        "ok"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn payout_without_token_is_unauthorized() {
    let base = spawn("noauth", &[]).await;
    let resp = client()
        .post(format!("{base}/payout"))
        .json(&serde_json::json!({ "message": "x" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 401);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn disallowed_origin_post_is_forbidden() {
    let base = spawn("origin", &["http://allowed.example"]).await;
    let resp = client()
        .post(format!("{base}/payout"))
        .header("Authorization", format!("Bearer {TOKEN}"))
        .header("Origin", "http://evil.example")
        .json(&serde_json::json!({ "message": "x" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 403);
}

/// The path I was least confident about: a real browser CORS preflight
/// (`OPTIONS` with `Access-Control-Request-Method`) must be answered by the
/// CORS layer *ahead of* the auth guard. An allowed origin gets the
/// `Access-Control-Allow-Origin` header echoed back.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cors_preflight_allowed_origin_is_approved() {
    let base = spawn("preflight-ok", &["http://app.example"]).await;
    let resp = client()
        .request(reqwest::Method::OPTIONS, format!("{base}/payout"))
        .header("Origin", "http://app.example")
        .header("Access-Control-Request-Method", "POST")
        .header(
            "Access-Control-Request-Headers",
            "authorization,content-type",
        )
        .send()
        .await
        .unwrap();
    assert!(
        resp.status().is_success(),
        "preflight status: {}",
        resp.status()
    );
    assert_eq!(
        resp.headers()
            .get("access-control-allow-origin")
            .and_then(|v| v.to_str().ok()),
        Some("http://app.example"),
        "allowed origin must be echoed in the preflight response"
    );
}

/// A preflight from a disallowed origin must NOT receive the allow-origin
/// header, so the browser blocks the real request.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cors_preflight_disallowed_origin_has_no_allow_header() {
    let base = spawn("preflight-bad", &["http://app.example"]).await;
    let resp = client()
        .request(reqwest::Method::OPTIONS, format!("{base}/payout"))
        .header("Origin", "http://evil.example")
        .header("Access-Control-Request-Method", "POST")
        .send()
        .await
        .unwrap();
    assert!(
        resp.headers().get("access-control-allow-origin").is_none(),
        "disallowed origin must not get an allow-origin header"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn payout_with_offer_signs_offline_and_hides_seed() {
    let base = spawn("payout", &[]).await;
    let message = format!("Configure OCEAN payout to {MOCK_OFFER} at block 840000");
    let resp = client()
        .post(format!("{base}/payout"))
        .header("Authorization", format!("Bearer {TOKEN}"))
        .json(&serde_json::json!({ "message": message, "offer": MOCK_OFFER }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body = resp.text().await.unwrap();
    // The seed must never appear in the response.
    assert!(
        !body.contains("music mystery deliver"),
        "response leaked the seed: {body}"
    );
    let v: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(v["offer"], MOCK_OFFER);
    assert_eq!(v["message"], message);
    assert!(v["address"].as_str().unwrap().starts_with("bc1q"));
    assert!(!v["signature"].as_str().unwrap().is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn offer_endpoint_uses_wallet_provider() {
    let base = spawn("offer", &[]).await;
    let resp = client()
        .post(format!("{base}/offer"))
        .header("Authorization", format!("Bearer {TOKEN}"))
        .json(&serde_json::json!({ "description": "OCEAN payout" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let v: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(v["offer"], MOCK_PROVISIONED_OFFER);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn create_offer_preserves_existing_primary() {
    // A primary offer is already persisted (onboarding). Minting another offer
    // (Profile "New offer") must NOT overwrite it, or a restart would restore a
    // secondary offer as the primary payout offer.
    let seed = write_seed("preserve-primary");
    let offer_file = seed.with_extension("offer");
    std::fs::write(&offer_file, "lno1primaryfromonboarding").unwrap();

    let base = spawn_with(SeedSource::File(seed.clone()), &[]).await;
    let resp = client()
        .post(format!("{base}/offer"))
        .header("Authorization", format!("Bearer {TOKEN}"))
        .json(&serde_json::json!({ "description": "an extra offer" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    assert_eq!(
        std::fs::read_to_string(&offer_file).unwrap().trim(),
        "lno1primaryfromonboarding",
        "minting an extra offer must not overwrite the persisted primary"
    );

    let _ = std::fs::remove_file(&seed);
    let _ = std::fs::remove_file(&offer_file);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn init_endpoint_provisions_and_returns_address() {
    let base = spawn("init", &[]).await;
    let resp = client()
        .post(format!("{base}/init"))
        .header("Authorization", format!("Bearer {TOKEN}"))
        .json(&serde_json::json!({}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let v: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(v["provisioned"], true);
    // Deterministic BIP84 address for TEST_MNEMONIC; never the seed.
    assert_eq!(
        v["mining_address"],
        "bc1qpstw48j7j9gjugw25jmjvd96jlwgdnedk5pr6r"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn generate_creates_wallet_and_reveals_phrase_once() {
    let (base, seed_path) = spawn_no_seed("generate", &[]).await;
    assert!(!seed_path.exists(), "precondition: no seed file");
    let resp = client()
        .post(format!("{base}/generate"))
        .header("Authorization", format!("Bearer {TOKEN}"))
        .json(&serde_json::json!({}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let v: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(
        v["mnemonic"].as_str().unwrap().split_whitespace().count(),
        24,
        "a fresh 24-word phrase is revealed once"
    );
    assert!(v["mining_address"].as_str().unwrap().starts_with("bc1q"));
    assert!(
        seed_path.exists(),
        "the phrase was persisted to the seed file"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn generate_refuses_to_clobber_existing_seed() {
    // `spawn` writes a seed file first, so /generate must 409. There is no
    // `force` on /generate (it would destroy the wallet) — sending one must be
    // ignored, so this still 409s and must NOT reveal a new phrase.
    let base = spawn("generate-conflict", &[]).await;
    let resp = client()
        .post(format!("{base}/generate"))
        .header("Authorization", format!("Bearer {TOKEN}"))
        .json(&serde_json::json!({ "force": true }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 409);
    let v: serde_json::Value = resp.json().await.unwrap();
    assert!(
        v.get("mnemonic").is_none(),
        "must not reveal a phrase on conflict"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn import_valid_phrase_returns_address() {
    let (base, seed_path) = spawn_no_seed("import", &[]).await;
    let resp = client()
        .post(format!("{base}/import"))
        .header("Authorization", format!("Bearer {TOKEN}"))
        .json(&serde_json::json!({ "mnemonic": TEST_MNEMONIC }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let v: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(
        v["mining_address"],
        "bc1qpstw48j7j9gjugw25jmjvd96jlwgdnedk5pr6r"
    );
    assert!(seed_path.exists());
    // The imported phrase is not echoed back.
    assert!(v.get("mnemonic").is_none());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn import_invalid_phrase_is_bad_request() {
    let (base, _) = spawn_no_seed("import-bad", &[]).await;
    let resp = client()
        .post(format!("{base}/import"))
        .header("Authorization", format!("Bearer {TOKEN}"))
        .json(&serde_json::json!({ "mnemonic": "abandon abandon abandon" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 400);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn payout_with_wrong_token_is_unauthorized() {
    // A present-but-wrong bearer token must be rejected (the constant-time
    // compare returns false), not just a missing one.
    let base = spawn("badtoken", &[]).await;
    let resp = client()
        .post(format!("{base}/payout"))
        .header("Authorization", "Bearer not-the-real-token")
        .json(&serde_json::json!({ "message": "x" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 401);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn payout_with_nonloopback_host_is_forbidden() {
    // DNS-rebinding defense end-to-end: a request whose Host header names a
    // non-loopback host is rejected by the guard even with a valid token,
    // before the handler runs. (The unit test covers `host_is_loopback`; this
    // exercises the guard wiring on a real socket.)
    let base = spawn("badhost", &[]).await;
    let resp = client()
        .post(format!("{base}/payout"))
        .header("Authorization", format!("Bearer {TOKEN}"))
        .header("Host", "evil.example")
        .json(&serde_json::json!({ "message": "x" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 403);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn generate_requires_token() {
    let (base, _) = spawn_no_seed("generate-noauth", &[]).await;
    let resp = client()
        .post(format!("{base}/generate"))
        .json(&serde_json::json!({}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 401);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn status_reports_configured_wallet_offline() {
    // A configured wallet: /status returns the derived address with no Lexe call,
    // so a frontend can skip onboarding on launch. Seed must not leak.
    let base = spawn("status-configured", &[]).await;
    let resp = client()
        .get(format!("{base}/status"))
        .header("Authorization", format!("Bearer {TOKEN}"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body = resp.text().await.unwrap();
    assert!(
        !body.contains("music mystery deliver"),
        "status leaked the seed"
    );
    let v: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(v["configured"], true);
    assert_eq!(
        v["mining_address"],
        "bc1qpstw48j7j9gjugw25jmjvd96jlwgdnedk5pr6r"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn status_reports_unconfigured_without_seed() {
    // A fresh install (no seed yet) reports unconfigured, so onboarding still runs.
    let (base, _) = spawn_no_seed("status-empty", &[]).await;
    let resp = client()
        .get(format!("{base}/status"))
        .header("Authorization", format!("Bearer {TOKEN}"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let v: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(v["configured"], false);
    assert_eq!(v["mining_address"], serde_json::Value::Null);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn forced_import_clears_stale_offer() {
    // A wallet with a persisted offer, then a force-import of a DIFFERENT seed:
    // the old offer belonged to the previous wallet, so it must be removed (else
    // status() would pair the new address with the stale offer).
    let seed = write_seed("force-clears-offer");
    let offer_file = seed.with_extension("offer");
    std::fs::write(&offer_file, "lno1staleofferfromoldwallet").unwrap();
    assert!(offer_file.exists());

    let base = spawn_with(SeedSource::File(seed.clone()), &[]).await;
    let resp = client()
        .post(format!("{base}/import"))
        .header("Authorization", format!("Bearer {TOKEN}"))
        .json(&serde_json::json!({ "mnemonic": OTHER_MNEMONIC, "force": true }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    assert!(
        !offer_file.exists(),
        "stale offer must be removed when the seed is force-replaced"
    );

    let _ = std::fs::remove_file(&seed);
}

// ── /payouts ───────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn payouts_endpoint_returns_wallet_provider_rows() {
    // MockWallet returns an empty Vec — proves the route is wired,
    // auth-guarded, and serializes through to JSON `[]`. The OCEAN
    // payer_note filter that produces the rows is exercised by unit
    // tests in `oceanln_common::lexe_wallet`.
    let base = spawn("payouts", &[]).await;
    let resp = client()
        .get(format!("{base}/payouts"))
        .header("Authorization", format!("Bearer {TOKEN}"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let v: serde_json::Value = resp.json().await.unwrap();
    assert!(v.is_array(), "payouts must serialize as a JSON array");
    assert_eq!(v.as_array().unwrap().len(), 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn payouts_endpoint_requires_bearer() {
    // No `Authorization` header → 401, same guard as every other
    // protected endpoint. Regression check: the route must be mounted
    // under the protected sub-router, not the public one.
    let base = spawn("payouts-unauthed", &[]).await;
    let resp = client()
        .get(format!("{base}/payouts"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 401);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn payouts_endpoint_accepts_limit_query() {
    // `?limit=N` must parse without 400-ing — the wrapper hands the
    // value straight to `list_offer_payouts`. MockWallet ignores it,
    // so the assertion is just that the route accepts the query.
    let base = spawn("payouts-limit", &[]).await;
    let resp = client()
        .get(format!("{base}/payouts?limit=42"))
        .header("Authorization", format!("Bearer {TOKEN}"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
}
