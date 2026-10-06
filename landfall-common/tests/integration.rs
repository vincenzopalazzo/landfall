//! Integration tests against an in-process axum mock sidecar.
//!
//! Scoped to the `payout` flow's single sidecar dependency:
//! `POST /v2/node/create_offer`.

use axum::http::HeaderMap;
use axum::routing::post;
use axum::{Json, Router};
use landfall_common::client::{CreateOfferReq, SidecarClient};
use landfall_common::error::Error;
use serde_json::{json, Value};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::Mutex;

// ── mock sidecar ────────────────────────────────────────────────

async fn create_offer_handler(Json(_body): Json<Value>) -> Json<Value> {
    Json(json!({
        "offer": "lno1qgsqvgnwgcg35z6ee2h3yczraddm72xrfua9uve2rlrm9deu7xyfzrcgqp0s"
    }))
}

async fn spawn_mock() -> SocketAddr {
    let app = Router::new().route("/v2/node/create_offer", post(create_offer_handler));

    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("local_addr");
    tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });
    addr
}

fn url_for(addr: SocketAddr) -> String {
    format!("http://{addr}")
}

fn offer_req() -> CreateOfferReq<'static> {
    CreateOfferReq {
        description: Some("OCEAN payout"),
        min_amount: Some("1000"),
    }
}

// ── tests ───────────────────────────────────────────────────────

#[tokio::test]
async fn create_offer_returns_payable_offer() {
    let addr = spawn_mock().await;
    let client = SidecarClient::new(url_for(addr), None).expect("client");
    let o = client
        .create_offer(offer_req())
        .await
        .expect("create_offer");
    assert!(o.offer.starts_with("lno1"));
}

/// When nothing is listening on the URL, the client must surface the
/// actionable `SidecarUnreachable` variant — not the generic `Http` —
/// so the user sees a message naming the binary they need to launch.
#[tokio::test]
async fn connection_refused_is_classified() {
    // Bind a port, grab the addr, then drop the listener so the port
    // is closed when we connect. Race-free way to get a guaranteed-dead
    // local address without picking a magic number.
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("local_addr");
    drop(listener);

    let client = SidecarClient::new(url_for(addr), None).expect("client");
    match client.create_offer(offer_req()).await {
        Err(Error::SidecarUnreachable { url, .. }) => {
            assert!(url.contains(&addr.to_string()));
        }
        other => panic!("expected SidecarUnreachable, got {other:?}"),
    }
}

/// A non-existent route (no `create_offer` on this version) must surface
/// as a classified `Api` error, not a panic.
#[tokio::test]
async fn missing_endpoint_returns_api_error() {
    let app = Router::new();
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("local_addr");
    tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });

    let client = SidecarClient::new(url_for(addr), None).expect("client");
    match client.create_offer(offer_req()).await {
        Err(Error::Api { code: 404, .. }) => {}
        other => panic!("expected Api 404, got {other:?}"),
    }
}

#[tokio::test]
async fn sends_bearer_credentials() {
    let captured: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
    let cap = captured.clone();

    let app = Router::new().route(
        "/v2/node/create_offer",
        post(move |hdrs: HeaderMap, Json(_b): Json<Value>| {
            let cap = cap.clone();
            async move {
                let v = hdrs
                    .get("authorization")
                    .and_then(|v| v.to_str().ok())
                    .map(String::from);
                *cap.lock().await = v;
                Json(json!({ "offer": "lno1qgsqvgnwgcg35z6ee2h3yczraddm72xrfua9uve2rlrm9deu7xyfzrcgqp0s" }))
            }
        }),
    );

    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("local_addr");
    tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });

    let client =
        SidecarClient::new(url_for(addr), Some("secret-token-123".into())).expect("client");
    client
        .create_offer(offer_req())
        .await
        .expect("create_offer");

    assert_eq!(
        captured.lock().await.as_deref(),
        Some("Bearer secret-token-123")
    );
}
