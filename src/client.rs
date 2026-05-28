//! Lexe sidecar HTTP client.
//!
//! Mirrors the endpoint surface used by the previous Zig client
//! (`../../lexe-zig-sdk/src/client.zig`) and the mock sidecar
//! fixture (`src/mock_sidecar.zig`).

use crate::error::{Error, Result};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use reqwest::{Client, Method, RequestBuilder, StatusCode};
use serde::{Deserialize, Serialize};

pub const DEFAULT_BASE_URL: &str = "http://127.0.0.1:5393";

pub struct SidecarClient {
    http: Client,
    base_url: String,
    credentials: Option<String>,
}

// ── Response types ──────────────────────────────────────────────

#[derive(Debug, Deserialize, Serialize)]
pub struct Health {
    pub status: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct NodeInfo {
    pub version: String,
    #[serde(default)]
    pub measurement: String,
    pub user_pk: String,
    pub node_pk: String,
    pub balance: String,
    pub lightning_balance: String,
    pub lightning_sendable_balance: String,
    #[serde(default)]
    pub lightning_max_sendable_balance: String,
    pub onchain_balance: String,
    #[serde(default)]
    pub onchain_trusted_balance: String,
    pub num_channels: u32,
    pub num_usable_channels: u32,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct OfferResp {
    pub offer: String,
}

#[derive(Debug, Serialize)]
pub struct CreateInvoiceReq<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub amount: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expiration_secs: Option<u64>,
    /// Note exposed in the offer/invoice for the payer to see.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payer_note: Option<&'a str>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Invoice {
    pub index: String,
    pub invoice: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub amount: Option<String>,
    pub created_at: i64,
    pub expires_at: i64,
    pub payment_hash: String,
    #[serde(default)]
    pub payment_secret: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PayInvoiceReq<'a> {
    pub invoice: &'a str,
    /// Used for amountless invoices to supply the amount in sats.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fallback_amount: Option<&'a str>,
    /// Personal note attached to the local payment record.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<&'a str>,
    /// Note delivered to the recipient (offers / LNURL-pay).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payer_note: Option<&'a str>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PayResp {
    pub index: String,
    pub created_at: i64,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Payment {
    pub index: String,
    pub rail: String,
    pub kind: String,
    pub direction: String,
    #[serde(default)]
    pub txid: Option<String>,
    #[serde(default)]
    pub amount: Option<String>,
    pub fees: String,
    pub status: String,
    pub status_msg: String,
    #[serde(default)]
    pub invoice: Option<String>,
    #[serde(default)]
    pub address: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
    #[serde(default)]
    pub payer_name: Option<String>,
    #[serde(default)]
    pub payer_note: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    #[serde(default)]
    pub expires_at: Option<i64>,
    #[serde(default)]
    pub finalized_at: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct ApiErrorBody {
    code: u16,
    msg: String,
}

// ── Client ──────────────────────────────────────────────────────

impl SidecarClient {
    pub fn new(base_url: String, credentials: Option<String>) -> Result<Self> {
        let http = Client::builder().build()?;
        Ok(Self {
            http,
            base_url,
            credentials,
        })
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url.trim_end_matches('/'), path)
    }

    fn headers(&self) -> HeaderMap {
        let mut h = HeaderMap::new();
        h.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        if let Some(creds) = &self.credentials {
            if let Ok(v) = HeaderValue::from_str(&format!("Bearer {creds}")) {
                h.insert(AUTHORIZATION, v);
            }
        }
        h
    }

    async fn send<T: for<'de> Deserialize<'de>>(&self, req: RequestBuilder) -> Result<T> {
        let resp = req
            .headers(self.headers())
            .send()
            .await
            .map_err(|e| self.classify_send_error(e))?;
        let status = resp.status();
        let bytes = resp.bytes().await?;
        if status.is_success() {
            return serde_json::from_slice(&bytes).map_err(Error::Json);
        }
        Err(api_error(status, &bytes))
    }

    /// Variant that returns `Ok(None)` on a bare HTTP 404 (used by
    /// `payment` lookup, where the sidecar uses 404 to signal "no such
    /// payment" rather than wrapping it in a JSON envelope).
    async fn send_opt<T: for<'de> Deserialize<'de>>(
        &self,
        req: RequestBuilder,
    ) -> Result<Option<T>> {
        let resp = req
            .headers(self.headers())
            .send()
            .await
            .map_err(|e| self.classify_send_error(e))?;
        let status = resp.status();
        if status == StatusCode::NOT_FOUND {
            return Ok(None);
        }
        let bytes = resp.bytes().await?;
        if status.is_success() {
            return serde_json::from_slice(&bytes)
                .map(Some)
                .map_err(Error::Json);
        }
        Err(api_error(status, &bytes))
    }

    /// Map low-level reqwest errors into actionable user-facing errors.
    /// In particular, distinguish "sidecar isn't running" from other
    /// HTTP failures so the message can name the binary the user needs
    /// to launch.
    fn classify_send_error(&self, e: reqwest::Error) -> Error {
        if e.is_connect() || e.is_timeout() {
            return Error::SidecarUnreachable {
                url: self.base_url.clone(),
                source: e,
            };
        }
        Error::Http(e)
    }

    pub async fn health(&self) -> Result<Health> {
        self.send(self.http.request(Method::GET, self.url("/v2/health")))
            .await
    }

    pub async fn node_info(&self) -> Result<NodeInfo> {
        self.send(
            self.http
                .request(Method::GET, self.url("/v2/node/node_info")),
        )
        .await
    }

    /// Fetch the BOLT12 offer from the node.
    ///
    /// Note: as of `sdk-sidecar` v0.4.x this endpoint is not served —
    /// the sidecar replies with "Client requested a non-existent
    /// endpoint" if called. Kept here so that when/if the endpoint
    /// lands upstream the client doesn't need a new release. Callers
    /// should treat this as best-effort and require the user to supply
    /// the offer manually.
    pub async fn offer(&self) -> Result<Option<OfferResp>> {
        self.send_opt(self.http.request(Method::GET, self.url("/v2/node/offer")))
            .await
    }

    pub async fn create_invoice(&self, req: CreateInvoiceReq<'_>) -> Result<Invoice> {
        self.send(
            self.http
                .request(Method::POST, self.url("/v2/node/create_invoice"))
                .json(&req),
        )
        .await
    }

    pub async fn pay_invoice(&self, req: PayInvoiceReq<'_>) -> Result<PayResp> {
        self.send(
            self.http
                .request(Method::POST, self.url("/v2/node/pay_invoice"))
                .json(&req),
        )
        .await
    }

    pub async fn payment(&self, index: &str) -> Result<Option<Payment>> {
        // Use reqwest's typed query so any special characters in `index`
        // are properly URL-encoded.
        self.send_opt(
            self.http
                .request(Method::GET, self.url("/v2/node/payment"))
                .query(&[("index", index)]),
        )
        .await
    }
}

fn api_error(status: StatusCode, bytes: &[u8]) -> Error {
    if let Ok(body) = serde_json::from_slice::<ApiErrorBody>(bytes) {
        Error::Api {
            code: body.code,
            msg: body.msg,
        }
    } else {
        Error::Api {
            code: status.as_u16(),
            msg: String::from_utf8_lossy(bytes).into_owned(),
        }
    }
}
