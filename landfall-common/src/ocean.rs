//! Read-only Rust client for OCEAN's public REST API (`https://api.ocean.xyz/v1`).
//!
//! Mirrors the TypeScript client in [`landfall-web/src/lib/ocean.ts`] — same
//! endpoints, same field shapes (all numeric fields arrive as strings on the
//! wire), same `{ "result": T } | { "error": "…" }` envelope. The frontend
//! talks to OCEAN directly from the browser (CORS-OK); this client exists so
//! Rust callers — the MCP server in `landfall-mcp`, future CLI subcommands,
//! anything else — query the same data without duplicating the parsing.
//!
//! Construct one [`OceanClient`] per process and reuse it: it wraps a
//! `reqwest::Client` which carries an internal connection pool, so repeated
//! calls share TCP/TLS sessions instead of opening a new one each time.

use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

/// Public OCEAN REST base URL. Hardcoded — there's no staging counterpart.
pub const OCEAN_API_BASE: &str = "https://api.ocean.xyz/v1";

/// A single user-stats snapshot from `/v1/statsnap/<address>`.
///
/// OCEAN returns every numeric field as a string (BTC amounts as decimal
/// strings, hashes/sec as integer strings, timestamps as unix-seconds
/// strings). We keep them as `String` and let callers coerce — same posture
/// as the frontend `StatSnap` interface.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StatSnap {
    pub snap_type: String,
    pub snap_ts: String,
    pub shares_60s: String,
    pub shares_300s: String,
    pub hashrate_60s: String,
    pub hashrate_300s: String,
    /// `lastest_share_ts` (sic — OCEAN's spelling).
    pub lastest_share_ts: String,
    pub shares_in_tides: String,
    pub estimated_earn_next_block: String,
    pub estimated_bonus_earn_next_block: String,
    pub estimated_total_earn_next_block: String,
    pub estimated_payout_next_block: String,
    pub unpaid: String,
}

/// Per-address hashrate windows + live worker count from
/// `/v1/user_hashrate/<address>`. Richer than `StatSnap`'s 60s/300s windows.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UserHashrate {
    pub snap_ts: String,
    pub db_ts: String,
    pub hashrate_60s: String,
    pub hashrate_300s: String,
    pub hashrate_600s: String,
    pub hashrate_1800s: String,
    pub hashrate_3600s: String,
    pub hashrate_10800s: String,
    pub hashrate_43200s: String,
    pub hashrate_86400s: String,
    pub active_worker_count: u64,
    pub lastest_share_ts: String,
}

/// One historical per-block earning credit for the address.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Earning {
    pub block_hash: String,
    /// ISO-8601 datetime without a timezone marker (treat as UTC).
    pub ts: String,
    /// OCEAN serializes these numeric fields inconsistently between
    /// endpoints — sometimes as JSON numbers, sometimes as
    /// quoted strings (per the frontend's `num()` coercion in
    /// `ocean.ts`). `deserialize_with` accepts both shapes; an
    /// unparseable string yields `None` rather than rejecting the
    /// entire `/earnpay` payload (which would block all the other
    /// earnings rows + the payouts list from rendering).
    #[serde(default, deserialize_with = "u64_or_str_opt")]
    pub shares_in_window: Option<u64>,
    #[serde(default, deserialize_with = "u64_or_str_opt")]
    pub fees_colected_satoshis: Option<u64>,
    #[serde(default, deserialize_with = "u64_or_str_opt")]
    pub satoshis_net_earned: Option<u64>,
}

/// Accept either a JSON number, a numeric string, or null/missing and
/// produce an `Option<u64>`. Lossy on overflow (returns `None`) so a
/// surprising upstream change can't crash the request.
fn u64_or_str_opt<'de, D: serde::Deserializer<'de>>(
    de: D,
) -> std::result::Result<Option<u64>, D::Error> {
    use serde::de::Error as _;
    let v: Option<serde_json::Value> = Option::deserialize(de)?;
    match v {
        None | Some(serde_json::Value::Null) => Ok(None),
        Some(serde_json::Value::Number(n)) => Ok(n.as_u64()),
        Some(serde_json::Value::String(s)) => Ok(s.parse::<u64>().ok()),
        Some(other) => Err(D::Error::custom(format!(
            "expected number, numeric string, or null; got {other:?}"
        ))),
    }
}

/// One past payout row from `/v1/earnpay/<address>`. Note OCEAN currently
/// returns Lightning payouts in the CSV-only `/data/csv/<addr>/payouts`
/// endpoint, NOT here — `payouts` may be empty even when LN payouts exist
/// upstream. Use `landfall_common::lexe_wallet::list_offer_payouts` for the
/// authoritative LN list (reads from the user's own Lexe node).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Payout {
    pub ts: serde_json::Value,
    #[serde(default)]
    pub on_chain_txid: Option<String>,
    pub total_satoshis_net_paid: serde_json::Value,
    #[serde(default)]
    pub is_generation_txn: bool,
}

/// `/v1/earnpay/<address>` response body.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EarnPay {
    pub start_ts: serde_json::Value,
    pub end_ts: serde_json::Value,
    #[serde(default)]
    pub earnings: Vec<Earning>,
    #[serde(default)]
    pub payouts: Vec<Payout>,
}

/// `/v1/pool_stat` response body — pool-wide context (network difficulty,
/// active miners, current TIDES window size, etc.).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PoolStat {
    pub snap_ts: String,
    pub active_users: String,
    pub active_workers: String,
    pub network_difficulty: String,
    #[serde(default)]
    pub current_tides_shares: String,
    #[serde(default)]
    pub max_tides_shares: String,
    #[serde(default)]
    pub current_estimated_block_reward: String,
}

/// OCEAN's response envelope: every endpoint returns `{ "result": T }` on
/// success or `{ "error": "<message>" }` on failure (regardless of HTTP
/// status). We treat anything else (e.g. HTML, empty body) as a protocol
/// error rather than handing the caller back a `T` deserialized from
/// nothing.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum Envelope<T> {
    Ok { result: T },
    Err { error: String },
}

/// Pooled, reusable client for `api.ocean.xyz/v1/*`. Cheap to clone (the
/// underlying `reqwest::Client` is `Arc<Inner>`).
#[derive(Debug, Clone)]
pub struct OceanClient {
    http: reqwest::Client,
    base: String,
}

impl Default for OceanClient {
    fn default() -> Self {
        // Default 15 s timeout — OCEAN's API is usually <500 ms. Long
        // enough to absorb a slow first connection on a fresh node, short
        // enough that a stuck request doesn't pile up MCP semaphore holds.
        let http = reqwest::Client::builder()
            .user_agent("landfall-mcp")
            .timeout(Duration::from_secs(15))
            .build()
            .expect("reqwest builder defaults always build");
        Self {
            http,
            base: OCEAN_API_BASE.to_string(),
        }
    }
}

impl OceanClient {
    /// Construct a client against a custom base URL (used by tests to point
    /// at a local mock). For production, use [`OceanClient::default`].
    pub fn with_base(base: impl Into<String>) -> Self {
        Self {
            base: base.into(),
            ..Self::default()
        }
    }

    async fn get<T: for<'de> Deserialize<'de>>(&self, path: &str) -> Result<T> {
        let url = format!("{}{}", self.base.trim_end_matches('/'), path);
        let resp = self
            .http
            .get(&url)
            .send()
            .await
            .map_err(|e| Error::Wallet(format!("ocean.xyz fetch {url} failed: {e}")))?;
        let status = resp.status();
        let body = resp
            .text()
            .await
            .map_err(|e| Error::Wallet(format!("ocean.xyz read body {url}: {e}")))?;
        let env: Envelope<T> = serde_json::from_str(&body).map_err(|e| {
            Error::Wallet(format!(
                "ocean.xyz {url} returned non-envelope JSON (status={status}): {e}; body: {}",
                truncate(&body, 200)
            ))
        })?;
        match env {
            Envelope::Ok { result } => Ok(result),
            Envelope::Err { error } => Err(Error::Wallet(format!(
                "ocean.xyz {url} returned error: {error}"
            ))),
        }
    }

    /// `/v1/statsnap/<address>`
    pub async fn statsnap(&self, address: &str) -> Result<StatSnap> {
        self.get(&format!("/statsnap/{}", urlencode(address))).await
    }

    /// `/v1/earnpay/<address>`
    pub async fn earnpay(&self, address: &str) -> Result<EarnPay> {
        self.get(&format!("/earnpay/{}", urlencode(address))).await
    }

    /// `/v1/user_hashrate/<address>`
    pub async fn user_hashrate(&self, address: &str) -> Result<UserHashrate> {
        self.get(&format!("/user_hashrate/{}", urlencode(address)))
            .await
    }

    /// `/v1/pool_stat`
    pub async fn pool_stat(&self) -> Result<PoolStat> {
        self.get("/pool_stat").await
    }
}

/// Bare-bones percent-encoder for the address path segment. OCEAN's
/// addresses are bech32 / base58 — no characters that actually need
/// encoding under RFC 3986 unreserved rules. Defensive anyway: anything
/// outside `A-Za-z0-9._~-` gets `%XX`-encoded.
fn urlencode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for byte in s.as_bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                out.push(*byte as char)
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

/// Truncate a string at a UTF-8 char boundary for inclusion in an error
/// message — `&body[..200]` panics in the middle of a multi-byte sequence.
fn truncate(s: &str, max: usize) -> String {
    let mut end = max.min(s.len());
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    s[..end].to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn envelope_parses_ok() {
        let body = r#"{"result":{"snap_ts":"1","active_users":"42","active_workers":"100","network_difficulty":"123"}}"#;
        let env: Envelope<PoolStat> = serde_json::from_str(body).unwrap();
        match env {
            Envelope::Ok { result } => assert_eq!(result.active_users, "42"),
            Envelope::Err { .. } => panic!("expected ok"),
        }
    }

    #[test]
    fn envelope_parses_err() {
        let body = r#"{"error":"No such user or user has no active workers"}"#;
        let env: Envelope<PoolStat> = serde_json::from_str(body).unwrap();
        match env {
            Envelope::Err { error } => assert!(error.contains("No such user")),
            Envelope::Ok { .. } => panic!("expected err"),
        }
    }

    #[test]
    fn statsnap_parses_a_real_payload() {
        // Mirror of the fixture in landfall-web/src/lib/Dashboard.test.ts.
        let body = r#"{"result":{
            "snap_type":"user","snap_ts":"1700000000",
            "shares_60s":"0","shares_300s":"0",
            "hashrate_60s":"0","hashrate_300s":"2000000000000",
            "lastest_share_ts":"1700000000","shares_in_tides":"262144",
            "estimated_earn_next_block":"0.00015",
            "estimated_bonus_earn_next_block":"0",
            "estimated_total_earn_next_block":"0.00015",
            "estimated_payout_next_block":"0.0002",
            "unpaid":"0.001"
        }}"#;
        let env: Envelope<StatSnap> = serde_json::from_str(body).unwrap();
        let s = match env {
            Envelope::Ok { result } => result,
            Envelope::Err { .. } => panic!("expected ok"),
        };
        assert_eq!(s.unpaid, "0.001");
        assert_eq!(s.shares_in_tides, "262144");
    }

    #[test]
    fn urlencode_passes_alnum_and_dash() {
        assert_eq!(urlencode("bc1qabcdef0123"), "bc1qabcdef0123");
        assert_eq!(
            urlencode("path-with.dots_and~tilde"),
            "path-with.dots_and~tilde"
        );
    }

    #[test]
    fn urlencode_escapes_special() {
        assert_eq!(urlencode("a/b"), "a%2Fb");
        assert_eq!(urlencode("a b"), "a%20b");
        assert_eq!(urlencode("a&b"), "a%26b");
    }

    #[test]
    fn truncate_respects_utf8_boundaries() {
        // 4-byte emoji at the cutoff: must NOT panic, must back off to a
        // char boundary.
        let s = "abc\u{1F600}xyz";
        let t = truncate(s, 4); // 4 is mid-emoji
        assert!(t.len() <= 4);
        assert!(s.starts_with(&t));
    }

    #[test]
    fn earning_accepts_number_or_string_amount() {
        // OCEAN serializes `satoshis_net_earned` as a JSON number in some
        // payloads and a quoted string in others — both must parse so a
        // single mixed-shape row doesn't reject the whole /earnpay
        // response. Regression for Codex review on PR D.
        let as_number =
            r#"{"block_hash":"abc","ts":"2026-01-01T00:00:00","satoshis_net_earned":42}"#;
        let as_string =
            r#"{"block_hash":"abc","ts":"2026-01-01T00:00:00","satoshis_net_earned":"42"}"#;
        let n: Earning = serde_json::from_str(as_number).unwrap();
        let s: Earning = serde_json::from_str(as_string).unwrap();
        assert_eq!(n.satoshis_net_earned, Some(42));
        assert_eq!(s.satoshis_net_earned, Some(42));
    }

    #[test]
    fn earning_tolerates_null_and_missing_fields() {
        // No numeric fields at all: missing → `None`, not an error.
        let empty = r#"{"block_hash":"abc","ts":"2026-01-01T00:00:00"}"#;
        let e: Earning = serde_json::from_str(empty).unwrap();
        assert_eq!(e.satoshis_net_earned, None);
        assert_eq!(e.fees_colected_satoshis, None);
        // Explicit null: also None.
        let nulled =
            r#"{"block_hash":"abc","ts":"2026-01-01T00:00:00","satoshis_net_earned":null}"#;
        let n: Earning = serde_json::from_str(nulled).unwrap();
        assert_eq!(n.satoshis_net_earned, None);
    }

    #[test]
    fn earning_returns_none_on_garbage_string() {
        // A non-numeric string field should NOT explode the whole row —
        // returning `None` lets every other field still arrive.
        let body = r#"{"block_hash":"abc","ts":"2026-01-01T00:00:00","satoshis_net_earned":"not-a-number"}"#;
        let e: Earning = serde_json::from_str(body).unwrap();
        assert_eq!(e.satoshis_net_earned, None);
    }
}
