//! Lexe sidecar HTTP client.
//!
//! Scoped to what the `payout` flow needs: creating a payable BOLT12 offer
//! on the node via `POST /v2/node/create_offer`.

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

// ── Request / response types ────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct CreateOfferReq<'a> {
    /// Human-readable description baked into the offer (and the invoices it
    /// produces). Lightning requires a description; `None` → empty string.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<&'a str>,
    /// Optional lower bound on the amount, as a string-encoded sat value.
    /// `None` → variable-amount offer (payer chooses).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_amount: Option<&'a str>,
}

/// Upstream `CreateOfferResponse`. The `Offer` newtype serializes as its
/// bech32 string, so a plain `String` decodes it.
#[derive(Debug, Deserialize, Serialize)]
pub struct CreateOfferResp {
    pub offer: String,
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

    /// Ask the node to create a payable BOLT12 offer.
    ///
    /// The node builds the offer with blinded paths back to itself, so the
    /// returned `lno1…` is actually payable (unlike one constructed locally
    /// from a derived key). Requires a sidecar version that serves
    /// `POST /v2/node/create_offer`.
    pub async fn create_offer(&self, req: CreateOfferReq<'_>) -> Result<CreateOfferResp> {
        self.send(
            self.http
                .request(Method::POST, self.url("/v2/node/create_offer"))
                .json(&req),
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
