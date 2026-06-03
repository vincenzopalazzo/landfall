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
}

/// Write the test seed to a uniquely-named 0600 temp file.
fn write_seed(name: &str) -> std::path::PathBuf {
    let path = std::env::temp_dir().join(format!("oceanln-httpd-it-{name}.seed"));
    std::fs::write(&path, TEST_MNEMONIC).expect("write seed file");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).expect("chmod 600");
    }
    path
}

/// Spawn the server on an ephemeral loopback port; returns its base URL.
async fn spawn(name: &str, allowed_origins: &[&str]) -> String {
    let state = Arc::new(AppState::new(
        ServerConfig {
            seed: SeedSource::File(write_seed(name)),
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
