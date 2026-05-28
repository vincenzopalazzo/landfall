//! Integration tests against an in-process axum mock sidecar.
//!
//! The fixture JSON values are lifted from the deleted Zig mock
//! (`src/mock_sidecar.zig:91-102`) so the contract stays identical:
//! a change here is a deliberate API change, not an accident.

use axum::extract::Query;
use axum::http::HeaderMap;
use axum::routing::{get, post};
use axum::{Json, Router};
use oceanln::client::{CreateInvoiceReq, PayInvoiceReq, SidecarClient};
use oceanln::error::Error;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::Mutex;

// ── mock sidecar ────────────────────────────────────────────────

async fn health_handler() -> Json<Value> {
    Json(json!({ "status": "ok" }))
}

async fn node_info_handler() -> Json<Value> {
    Json(json!({
        "version": "0.9.2-mock",
        "measurement": "aabb",
        "user_pk": "cc",
        "node_pk": "dd",
        "balance": "50000",
        "lightning_balance": "25000",
        "lightning_sendable_balance": "20000",
        "lightning_max_sendable_balance": "25000",
        "onchain_balance": "25000",
        "onchain_trusted_balance": "25000",
        "num_channels": 2,
        "num_usable_channels": 2,
    }))
}

async fn offer_handler() -> Json<Value> {
    Json(json!({
        "offer": "lno1qgsqvgnwgcg35z6ee2h3yczraddm72xrfua9uve2rlrm9deu7xyfzrcgqp0s"
    }))
}

async fn create_invoice_handler(Json(_body): Json<Value>) -> Json<Value> {
    Json(json!({
        "index": "0000001772349163844-ln_mock",
        "invoice": "lnbc10n1pnmockqqqsyqcyq5rqwzqfqqqsyqcyq5rqwzqf",
        "description": "mock invoice",
        "amount": "1000",
        "created_at": 1_772_349_163_844_i64,
        "expires_at": 1_772_352_763_844_i64,
        "payment_hash": "aabbccdd00112233aabbccdd00112233aabbccdd00112233aabbccdd00112233",
        "payment_secret": "11223344556677881122334455667788112233445566778811223344556677aa",
    }))
}

async fn payment_handler(Query(q): Query<HashMap<String, String>>) -> Json<Value> {
    let index = q.get("index").cloned().unwrap_or_default();
    Json(json!({
        "index": index,
        "rail": "invoice",
        "kind": "invoice",
        "direction": "inbound",
        "amount": "1000",
        "fees": "0",
        "status": "completed",
        "status_msg": "received",
        "created_at": 1_772_349_163_844_i64,
        "updated_at": 1_772_349_170_000_i64,
    }))
}

async fn spawn_mock() -> SocketAddr {
    let app = Router::new()
        .route("/v2/health", get(health_handler))
        .route("/v2/node/node_info", get(node_info_handler))
        .route("/v2/node/offer", get(offer_handler))
        .route("/v2/node/create_invoice", post(create_invoice_handler))
        .route("/v2/node/payment", get(payment_handler));

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

// ── tests ───────────────────────────────────────────────────────

// Ports of `src/integration_test.zig:214-309` (wallet section) — every
// scenario the Zig suite covered, run against the equivalent fixtures.

#[tokio::test]
async fn health_returns_ok() {
    let addr = spawn_mock().await;
    let client = SidecarClient::new(url_for(addr), None).expect("client");
    let h = client.health().await.expect("health");
    assert_eq!(h.status, "ok");
}

#[tokio::test]
async fn node_info_decodes_full_payload() {
    let addr = spawn_mock().await;
    let client = SidecarClient::new(url_for(addr), None).expect("client");
    let i = client.node_info().await.expect("node_info");
    assert_eq!(i.version, "0.9.2-mock");
    assert_eq!(i.balance, "50000");
    assert_eq!(i.lightning_balance, "25000");
    assert_eq!(i.onchain_balance, "25000");
    assert_eq!(i.num_channels, 2);
    assert_eq!(i.num_usable_channels, 2);
    assert_eq!(i.user_pk, "cc");
    assert_eq!(i.node_pk, "dd");
}

#[tokio::test]
async fn create_invoice_decodes_payload() {
    let addr = spawn_mock().await;
    let client = SidecarClient::new(url_for(addr), None).expect("client");
    let inv = client
        .create_invoice(CreateInvoiceReq {
            amount: "1000",
            description: Some("mock invoice"),
            expiration_secs: 3600,
        })
        .await
        .expect("create_invoice");
    assert_eq!(inv.amount.as_deref(), Some("1000"));
    assert_eq!(inv.description.as_deref(), Some("mock invoice"));
    assert!(inv.invoice.starts_with("lnbc"));
    assert_eq!(inv.payment_hash.len(), 64);
    assert!(inv.expires_at > inv.created_at);
}

#[tokio::test]
async fn payment_lookup_decodes_payload() {
    let addr = spawn_mock().await;
    let client = SidecarClient::new(url_for(addr), None).expect("client");
    let p = client
        .payment("0000001772349163844-ln_mock")
        .await
        .expect("payment")
        .expect("Some(payment)");
    assert_eq!(p.index, "0000001772349163844-ln_mock");
    assert_eq!(p.rail, "invoice");
    assert_eq!(p.kind, "invoice");
    assert_eq!(p.direction, "inbound");
    assert_eq!(p.status, "completed");
    assert_eq!(p.amount.as_deref(), Some("1000"));
    assert_eq!(p.fees, "0");
}

#[tokio::test]
async fn offer_returns_known_offer() {
    let addr = spawn_mock().await;
    let client = SidecarClient::new(url_for(addr), None).expect("client");
    let o = client.offer().await.expect("offer").expect("Some(offer)");
    assert!(o.offer.starts_with("lno1"));
}

#[tokio::test]
async fn unknown_endpoint_returns_404() {
    let addr = spawn_mock().await;
    let client = SidecarClient::new(url_for(addr), None).expect("client");
    let r = client
        .pay_invoice(PayInvoiceReq {
            invoice: "lnbc1...",
        })
        .await;
    match r {
        Err(Error::Api { code: 404, .. }) => {}
        other => panic!("expected Api 404, got {other:?}"),
    }
}

#[tokio::test]
async fn sends_bearer_credentials() {
    let captured: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
    let cap = captured.clone();

    let app = Router::new().route(
        "/v2/health",
        get(move |hdrs: HeaderMap| {
            let cap = cap.clone();
            async move {
                let v = hdrs
                    .get("authorization")
                    .and_then(|v| v.to_str().ok())
                    .map(String::from);
                *cap.lock().await = v;
                Json(json!({ "status": "ok" }))
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
    client.health().await.expect("health");

    assert_eq!(
        captured.lock().await.as_deref(),
        Some("Bearer secret-token-123")
    );
}

#[tokio::test]
async fn payment_index_is_url_encoded() {
    // A pathological index containing `&` and `=` must NOT split the
    // query string. The mock captures the raw `index` parameter.
    let captured: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
    let cap = captured.clone();

    let app = Router::new().route(
        "/v2/node/payment",
        get(move |Query(q): Query<HashMap<String, String>>| {
            let cap = cap.clone();
            async move {
                *cap.lock().await = q.get("index").cloned();
                Json(json!({
                    "index": q.get("index").cloned().unwrap_or_default(),
                    "rail": "invoice", "kind": "invoice", "direction": "inbound",
                    "fees": "0", "status": "completed", "status_msg": "x",
                    "created_at": 0_i64, "updated_at": 0_i64,
                }))
            }
        }),
    );

    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("local_addr");
    tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });

    let weird = "abc&injected=evil";
    let client = SidecarClient::new(url_for(addr), None).expect("client");
    let _ = client.payment(weird).await.expect("payment");

    assert_eq!(captured.lock().await.as_deref(), Some(weird));
}

#[tokio::test]
async fn payment_returns_none_for_404() {
    // Spawn a stub server that always 404s on /v2/node/payment.
    let app = Router::new();
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("local_addr");
    tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });

    let client = SidecarClient::new(url_for(addr), None).expect("client");
    let p = client.payment("nonexistent").await.expect("payment");
    assert!(p.is_none());
}
