//! oceanln-mcp — read-only MCP proxy in front of `oceanln-httpd`.
//!
//! This crate is a **protocol adapter**. Every tool handler is a 3-line
//! call to the local `oceanln-httpd` REST API. There is NO direct call
//! into `oceanln-common::service`, no `WalletProvider` trait usage, no
//! `lexe_wallet::*` call. The invariant is simple to audit: anything an
//! AI can do via MCP is exactly what a human can do via REST. If a
//! capability isn't a REST route on `oceanln-httpd`, MCP can't expose it.
//!
//! Run-shape (see `src/main.rs`):
//!
//! ```text
//!   oceanln-mcp --base http://127.0.0.1:7762 [--httpd-token T] --bind 127.0.0.1:7763
//! ```
//!
//! MCP clients (Goose, Claude Code, …) connect to this binary's
//! `--bind` port. Each tool call becomes one HTTP round-trip to
//! `--base`. Auth on the inner hop is optional: omit `--httpd-token`
//! when `oceanln-httpd` was started with `--no-auth`.
//!
//! Tools (all strictly read-only; same surface as the REST API):
//!
//! 1. `get_status`           → `GET /status`
//! 2. `list_payouts`         → `GET /payouts?limit=N`
//! 3. `get_ocean_stats`      → `GET /ocean/statsnap/<addr>`
//! 4. `get_ocean_hashrate`   → `GET /ocean/user_hashrate/<addr>`
//! 5. `get_ocean_earnings`   → `GET /ocean/earnpay/<addr>`
//! 6. `get_ocean_pool_stats` → `GET /ocean/pool_stat`
//! 7. `get_health`           → `GET /health`

use std::sync::Arc;
use std::time::Duration;

use oceanln_common::lexe_wallet::OceanPayout;
use oceanln_common::ocean::{EarnPay, PoolStat, StatSnap, UserHashrate};
use oceanln_common::service::StatusResp;
use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::handler::server::ServerHandler;
use rmcp::model::{
    CallToolResult, Content, ErrorData, Implementation, ServerCapabilities, ServerInfo,
};
use rmcp::{tool, tool_handler, tool_router};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// ── HTTP client targeting oceanln-httpd ──────────────────────────

/// Thin reqwest client targeting `oceanln-httpd`. Cheap to clone —
/// the underlying `reqwest::Client` is `Arc<Inner>`.
#[derive(Debug, Clone)]
pub struct HttpdClient {
    http: reqwest::Client,
    base: String,
    token: Option<String>,
}

impl HttpdClient {
    /// Construct a client. `token` is optional — when None, no bearer
    /// header is sent (httpd must have been started with `--no-auth`).
    /// Bounded 30-second timeout per call so a stalled inner request
    /// can't pin an MCP tool indefinitely.
    pub fn new(base: impl Into<String>, token: Option<String>) -> Self {
        let http = reqwest::Client::builder()
            .user_agent(concat!("oceanln-mcp/", env!("CARGO_PKG_VERSION")))
            .timeout(Duration::from_secs(30))
            .build()
            .expect("reqwest builder defaults always build");
        Self {
            http,
            base: base.into(),
            token,
        }
    }

    /// Perform a GET, attach the bearer if configured, parse the JSON
    /// response. Any non-2xx status surfaces as a Wallet error carrying
    /// the HTTP code + body excerpt (truncated to keep tool-call
    /// payloads small).
    async fn get<T: for<'de> Deserialize<'de>>(
        &self,
        path: &str,
    ) -> std::result::Result<T, oceanln_common::error::Error> {
        let url = format!("{}{}", self.base.trim_end_matches('/'), path);
        let mut req = self.http.get(&url);
        if let Some(t) = &self.token {
            req = req.bearer_auth(t);
        }
        let resp = req
            .send()
            .await
            .map_err(|e| oceanln_common::error::Error::Wallet(format!("{url}: {e}")))?;
        let status = resp.status();
        let body = resp
            .text()
            .await
            .map_err(|e| oceanln_common::error::Error::Wallet(format!("read {url}: {e}")))?;
        if !status.is_success() {
            return Err(oceanln_common::error::Error::Wallet(format!(
                "{url} → HTTP {status}: {}",
                truncate(&body, 300)
            )));
        }
        serde_json::from_str(&body).map_err(|e| {
            oceanln_common::error::Error::Wallet(format!(
                "{url} returned non-JSON (status={status}): {e}; body: {}",
                truncate(&body, 200)
            ))
        })
    }

    /// `GET /health` returns `{ "status": "ok" }`. We don't depend on the
    /// exact body — just that the call succeeds.
    pub async fn health(&self) -> std::result::Result<String, oceanln_common::error::Error> {
        let v: serde_json::Value = self.get("/health").await?;
        Ok(v.to_string())
    }

    pub async fn status(&self) -> std::result::Result<StatusResp, oceanln_common::error::Error> {
        self.get("/status").await
    }

    pub async fn payouts(
        &self,
        limit: Option<u16>,
    ) -> std::result::Result<Vec<OceanPayout>, oceanln_common::error::Error> {
        let q = limit.map(|n| format!("?limit={n}")).unwrap_or_default();
        self.get(&format!("/payouts{q}")).await
    }

    pub async fn ocean_statsnap(
        &self,
        address: &str,
    ) -> std::result::Result<StatSnap, oceanln_common::error::Error> {
        self.get(&format!("/ocean/statsnap/{}", urlencode(address)))
            .await
    }

    pub async fn ocean_earnpay(
        &self,
        address: &str,
    ) -> std::result::Result<EarnPay, oceanln_common::error::Error> {
        self.get(&format!("/ocean/earnpay/{}", urlencode(address)))
            .await
    }

    pub async fn ocean_user_hashrate(
        &self,
        address: &str,
    ) -> std::result::Result<UserHashrate, oceanln_common::error::Error> {
        self.get(&format!("/ocean/user_hashrate/{}", urlencode(address)))
            .await
    }

    pub async fn ocean_pool_stat(
        &self,
    ) -> std::result::Result<PoolStat, oceanln_common::error::Error> {
        self.get("/ocean/pool_stat").await
    }
}

// ── MCP service ──────────────────────────────────────────────────

/// The MCP service. Cheaply cloned: state lives in `Arc`s under
/// `HttpdClient`. One instance per session.
#[derive(Clone)]
pub struct OceanMcpService {
    client: HttpdClient,
    tool_router: ToolRouter<OceanMcpService>,
}

/// Args for `list_payouts`.
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct ListPayoutsArgs {
    /// Max OCEAN payouts to return. The httpd route paginates against
    /// Lexe's `MAX_PAYMENTS_BATCH_SIZE` (100) internally; values above
    /// 100 just fan out into multiple upstream round-trips. Defaults
    /// to 100.
    #[serde(default)]
    pub limit: Option<u16>,
}

/// Args for the four `get_ocean_*` per-address tools.
///
/// `address` is OPTIONAL: when the AI asks "show me my stats", it
/// shouldn't have to know the wallet's address first. If omitted, the
/// tool fetches `/status` from oceanln-httpd, takes the configured
/// mining address, and uses that. Errors with a clear message if the
/// wallet isn't configured yet.
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct AddressArgs {
    /// The Bitcoin address registered with OCEAN (mainnet P2WPKH bech32:
    /// `bc1q…`). OPTIONAL — omit to use the configured mining address
    /// from the local wallet (`get_status.address`).
    #[serde(default)]
    pub address: Option<String>,
}

/// AI-friendly mirror of [`StatusResp`] returned by the `get_status`
/// MCP tool. The REST shape uses `mining_address` (legacy field name
/// the web/CLI/Tauri callers already deserialize against); LLMs
/// reliably guess `address` first, so we present the MCP response
/// under that key. Borrowed strings — this struct is constructed
/// per-call from a freshly-fetched [`StatusResp`] and serialized
/// immediately, so no lifetime gymnastics for the caller.
#[derive(Debug, Serialize)]
struct McpStatus<'a> {
    /// True when the wallet seed exists. When false, `address` and
    /// `offer` are both null.
    configured: bool,
    /// The mining address derived from the wallet's seed (the one
    /// registered with OCEAN). This IS the address you pass to the
    /// `get_ocean_*` tools, or omit those tools' `address` arg and
    /// they'll resolve it from here automatically.
    address: Option<&'a str>,
    /// The persisted primary BOLT12 offer (`lno1…`) OCEAN sends
    /// Lightning payouts to. Null until the wizard creates one.
    offer: Option<&'a str>,
}

#[tool_router]
impl OceanMcpService {
    pub fn new(client: HttpdClient) -> Self {
        Self {
            client,
            tool_router: Self::tool_router(),
        }
    }

    /// Tool 1: local wallet + offer state.
    #[tool(
        description = "Return the configured mining address and BOLT12 offer for this wallet (if any). Response shape: { configured: bool, address: string|null, offer: string|null }. The `address` field IS the mining address — pass it as the `address` argument to get_ocean_stats / get_ocean_hashrate / get_ocean_earnings (or omit `address` on those tools and they will auto-resolve it from here). Proxies GET /status on oceanln-httpd."
    )]
    pub async fn get_status(&self) -> std::result::Result<CallToolResult, ErrorData> {
        let resp = self
            .client
            .status()
            .await
            .map_err(|e| internal(format!("status: {e}")))?;
        // The REST shape (`oceanln_common::service::StatusResp`) uses
        // `mining_address`, which an AI naturally tries to read as
        // `address`. Re-key here so the MCP-side response is what an LLM
        // would guess first — that prevented Goose from finding the
        // address in a recent live session (issue #11). The REST contract
        // stays as-is for web/CLI/Tauri callers.
        json_result(&McpStatus {
            configured: resp.configured,
            address: resp.mining_address.as_deref(),
            offer: resp.offer.as_deref(),
        })
    }

    /// Tool 2: OCEAN Lightning payout history.
    #[tool(
        description = "List OCEAN's Lightning payouts to this wallet's BOLT12 offer, newest first. Proxies GET /payouts on oceanln-httpd."
    )]
    pub async fn list_payouts(
        &self,
        Parameters(args): Parameters<ListPayoutsArgs>,
    ) -> std::result::Result<CallToolResult, ErrorData> {
        let resp = self
            .client
            .payouts(args.limit)
            .await
            .map_err(|e| internal(format!("list_payouts: {e}")))?;
        json_result(&resp)
    }

    /// Resolve the effective address for a per-address OCEAN tool: use the
    /// caller-supplied one if any, otherwise fall back to the wallet's
    /// configured mining address. Errors with a human-readable message when
    /// both are missing (wallet not provisioned and AI didn't supply one).
    async fn resolve_address(
        &self,
        supplied: Option<String>,
    ) -> std::result::Result<String, ErrorData> {
        if let Some(a) = supplied.filter(|s| !s.trim().is_empty()) {
            return Ok(a);
        }
        let status = self
            .client
            .status()
            .await
            .map_err(|e| internal(format!("status (auto-address lookup): {e}")))?;
        status.mining_address.ok_or_else(|| {
            internal(
                "no address: omit-arg fallback needs the wallet's configured mining \
                 address, but get_status reports the wallet is not yet configured. \
                 Either pass `address` explicitly or finish the wallet setup."
                    .to_string(),
            )
        })
    }

    /// Tool 3: OCEAN public statsnap.
    #[tool(
        description = "Fetch OCEAN's user statsnap: unpaid balance, TIDES shares, estimated next-block earnings, 60s/300s hashrate, last share time. The `address` argument is OPTIONAL — when omitted, the tool uses the wallet's own mining address (from get_status). Proxies GET /ocean/statsnap/<address> on oceanln-httpd."
    )]
    pub async fn get_ocean_stats(
        &self,
        Parameters(args): Parameters<AddressArgs>,
    ) -> std::result::Result<CallToolResult, ErrorData> {
        let address = self.resolve_address(args.address).await?;
        let resp = self
            .client
            .ocean_statsnap(&address)
            .await
            .map_err(|e| internal(format!("ocean.statsnap: {e}")))?;
        json_result(&resp)
    }

    /// Tool 4: OCEAN public hashrate windows.
    #[tool(
        description = "Fetch OCEAN's per-address hashrate across rolling windows (60s, 5m, 10m, 30m, 1h, 3h, 12h, 24h) plus the live active worker count. The `address` argument is OPTIONAL — when omitted, uses the wallet's own mining address (from get_status). Proxies GET /ocean/user_hashrate/<address>."
    )]
    pub async fn get_ocean_hashrate(
        &self,
        Parameters(args): Parameters<AddressArgs>,
    ) -> std::result::Result<CallToolResult, ErrorData> {
        let address = self.resolve_address(args.address).await?;
        let resp = self
            .client
            .ocean_user_hashrate(&address)
            .await
            .map_err(|e| internal(format!("ocean.user_hashrate: {e}")))?;
        json_result(&resp)
    }

    /// Tool 5: OCEAN public earnings + on-chain payouts.
    #[tool(
        description = "Fetch OCEAN's per-address earnings history (per-block credits) and on-chain payout list. The `address` argument is OPTIONAL — when omitted, uses the wallet's own mining address (from get_status). Note: Lightning payouts are NOT in this feed — use list_payouts. Proxies GET /ocean/earnpay/<address>."
    )]
    pub async fn get_ocean_earnings(
        &self,
        Parameters(args): Parameters<AddressArgs>,
    ) -> std::result::Result<CallToolResult, ErrorData> {
        let address = self.resolve_address(args.address).await?;
        let resp = self
            .client
            .ocean_earnpay(&address)
            .await
            .map_err(|e| internal(format!("ocean.earnpay: {e}")))?;
        json_result(&resp)
    }

    /// Tool 6: pool-wide stats.
    #[tool(
        description = "Fetch OCEAN's pool-wide stats: active users + workers, current network difficulty, TIDES window size, current estimated block reward. Proxies GET /ocean/pool_stat."
    )]
    pub async fn get_ocean_pool_stats(&self) -> std::result::Result<CallToolResult, ErrorData> {
        let resp = self
            .client
            .ocean_pool_stat()
            .await
            .map_err(|e| internal(format!("ocean.pool_stat: {e}")))?;
        json_result(&resp)
    }

    /// Tool 7: liveness probe — hits httpd's open `/health`. Returns the
    /// httpd payload so a caller can sanity-check the connection.
    #[tool(
        description = "Confirm both the MCP server and the upstream oceanln-httpd are reachable. Proxies GET /health on oceanln-httpd."
    )]
    pub async fn get_health(&self) -> std::result::Result<CallToolResult, ErrorData> {
        let resp = self
            .client
            .health()
            .await
            .map_err(|e| internal(format!("health: {e}")))?;
        Ok(CallToolResult::success(vec![Content::text(resp)]))
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for OceanMcpService {
    fn get_info(&self) -> ServerInfo {
        // `ServerInfo` and `Implementation` are both `#[non_exhaustive]`,
        // so struct-expression syntax is rejected. Mutate the default in
        // place, overriding the fields that matter: tools capability +
        // a clear server identity for MCP clients to render.
        let mut info = ServerInfo::default();
        let mut me = Implementation::default();
        me.name = env!("CARGO_PKG_NAME").to_string();
        me.version = env!("CARGO_PKG_VERSION").to_string();
        info.server_info = me;
        info.capabilities = ServerCapabilities::builder().enable_tools().build();
        info.instructions = Some(
            "Read-only MCP proxy for OCEAN Lightning payouts. Every tool \
             is a thin wrapper over a REST endpoint on oceanln-httpd; the \
             server itself touches no keys and signs nothing. Use \
             get_health first to confirm both this server AND the \
             upstream oceanln-httpd are reachable, then get_status to \
             see the configured mining address, then any of the \
             get_ocean_* tools for public OCEAN data, or list_payouts \
             for confirmed Lightning payouts."
                .to_string(),
        );
        info
    }
}

// ── helpers ──────────────────────────────────────────────────────

fn json_result<T: Serialize>(value: &T) -> std::result::Result<CallToolResult, ErrorData> {
    let json =
        serde_json::to_string_pretty(value).map_err(|e| internal(format!("serialize: {e}")))?;
    Ok(CallToolResult::success(vec![Content::text(json)]))
}

fn internal(message: String) -> ErrorData {
    ErrorData::internal_error(message, None)
}

/// Defensive percent-encoder for path segments. Bitcoin addresses are
/// bech32/base58 (no chars that actually need encoding), but a future
/// caller could pass something else; encode anything outside RFC 3986
/// unreserved.
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

/// Truncate a string at a UTF-8 char boundary so it's safe to include
/// in an error message — `&body[..N]` panics in the middle of a
/// multi-byte sequence.
fn truncate(s: &str, max: usize) -> String {
    let mut end = max.min(s.len());
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    s[..end].to_string()
}

/// Public constructor for a Streamable-HTTP MCP service over a given
/// `HttpdClient`. Exposed so `src/main.rs` can wire it into axum.
pub use rmcp::transport::streamable_http_server::{
    session::local::LocalSessionManager, StreamableHttpServerConfig, StreamableHttpService,
};

/// Build a `StreamableHttpService` ready to mount at any axum path.
/// `client` is shared across all sessions via the factory closure.
pub fn streamable_http_service(
    client: HttpdClient,
) -> StreamableHttpService<OceanMcpService, LocalSessionManager> {
    StreamableHttpService::new(
        {
            let c = Arc::new(client);
            move || Ok(OceanMcpService::new((*c).clone()))
        },
        Arc::new(LocalSessionManager::default()),
        StreamableHttpServerConfig::default(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn server_info_identifies_us() {
        let svc = OceanMcpService::new(HttpdClient::new("http://x", None));
        let info = svc.get_info();
        assert_eq!(info.server_info.name, env!("CARGO_PKG_NAME"));
        assert_eq!(info.server_info.version, env!("CARGO_PKG_VERSION"));
        assert!(info.capabilities.tools.is_some());
        assert!(info
            .instructions
            .as_ref()
            .is_some_and(|s| s.contains("oceanln-httpd")));
    }

    #[test]
    fn urlencode_passes_alnum() {
        assert_eq!(urlencode("bc1q-abc.0_~"), "bc1q-abc.0_~");
        assert_eq!(urlencode("a/b c&d"), "a%2Fb%20c%26d");
    }

    #[test]
    fn truncate_handles_utf8() {
        let s = "abc\u{1F600}xyz";
        let t = truncate(s, 4);
        assert!(t.len() <= 4);
        assert!(s.starts_with(&t));
    }

    /// `resolve_address` short-circuits before touching httpd when the
    /// caller supplies a non-empty address. We point the client at a
    /// guaranteed-unreachable URL so any HTTP call would fail — passing
    /// proves the supplied-arg path never reaches the network.
    #[tokio::test]
    async fn resolve_address_returns_supplied_without_network() {
        let svc = OceanMcpService::new(HttpdClient::new(
            "http://127.0.0.1:1", // port 1 = guaranteed-refused
            None,
        ));
        let got = svc
            .resolve_address(Some("bc1qfoo".to_string()))
            .await
            .expect("supplied address must short-circuit");
        assert_eq!(got, "bc1qfoo");
    }

    /// An empty / whitespace-only address is treated the same as None
    /// (Codex called this out on PR D: copy-paste from a UI field may
    /// produce ""). With a bad base URL the fallback fails → the call
    /// must reach the fallback branch (not return the empty string).
    #[tokio::test]
    async fn resolve_address_treats_empty_as_missing() {
        let svc = OceanMcpService::new(HttpdClient::new("http://127.0.0.1:1", None));
        for empty in ["", "   ", "\t\n"] {
            let err = svc
                .resolve_address(Some(empty.to_string()))
                .await
                .expect_err("empty address must fall through to /status fetch");
            // We can't easily assert on `ErrorData` internals across rmcp
            // versions; serializing covers the message path used by clients.
            let s = serde_json::to_string(&err).unwrap_or_default();
            assert!(
                s.contains("auto-address lookup") || s.contains("status"),
                "fallback error must mention the status lookup; got: {s}"
            );
        }
    }

    /// When no address is supplied AND httpd is unreachable, the user
    /// gets the fallback's status-fetch error (not a panic, not a
    /// silent empty string).
    #[tokio::test]
    async fn resolve_address_surfaces_status_error_when_missing() {
        let svc = OceanMcpService::new(HttpdClient::new("http://127.0.0.1:1", None));
        let err = svc
            .resolve_address(None)
            .await
            .expect_err("None must trigger /status fetch (which fails on unreachable host)");
        let s = serde_json::to_string(&err).unwrap_or_default();
        assert!(s.contains("auto-address lookup") || s.contains("status"));
    }

    /// `McpStatus` serializes with `address` as the top-level key, NOT
    /// `mining_address` — the whole point of issue #11's fix.
    #[test]
    fn mcp_status_uses_address_key() {
        let s = McpStatus {
            configured: true,
            address: Some("bc1qarc"),
            offer: Some("lno1pg7"),
        };
        let j = serde_json::to_string(&s).unwrap();
        assert!(j.contains("\"address\":\"bc1qarc\""), "got: {j}");
        assert!(j.contains("\"offer\":\"lno1pg7\""), "got: {j}");
        assert!(j.contains("\"configured\":true"), "got: {j}");
        // Regression: the REST contract's `mining_address` MUST NOT
        // leak through the MCP layer.
        assert!(
            !j.contains("mining_address"),
            "MCP response leaked the REST field name `mining_address`: {j}"
        );
    }

    /// Unconfigured wallet: `address` and `offer` serialize as JSON null,
    /// not omitted. The AI then sees `null` (and knows the wallet isn't
    /// set up) rather than missing keys (which it might misread).
    #[test]
    fn mcp_status_unconfigured_serializes_explicit_nulls() {
        let s = McpStatus {
            configured: false,
            address: None,
            offer: None,
        };
        let j = serde_json::to_string(&s).unwrap();
        assert!(j.contains("\"address\":null"), "got: {j}");
        assert!(j.contains("\"offer\":null"), "got: {j}");
        assert!(j.contains("\"configured\":false"), "got: {j}");
    }
}
