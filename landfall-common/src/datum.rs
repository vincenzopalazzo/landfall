//! Read-only Rust client for the miner's own DATUM Gateway JSON API
//! (`/v1/*`, DATUM Gateway 0.5.0 or newer).
//!
//! The gateway runs wherever the miner put it (same box, LAN, a VPS), so its
//! URL is a user setting persisted next to the seed ([`load_settings`] /
//! [`save_settings`]). It is never taken from a request: the transports read
//! it from disk per call, which keeps `landfall-httpd` from being an open
//! proxy to arbitrary hosts.
//!
//! The gateway sends no CORS headers, so a browser on another origin cannot
//! read its responses. Every frontend therefore goes through this client:
//! `landfall-httpd` serves it as `/datum/*` and the desktop shell as Tauri
//! commands, the same split as [`crate::ocean`].
//!
//! Only the endpoints that need no password are covered here. The gateway
//! answers `{"error": "…"}` on failure and HTML for unknown paths (any gateway
//! older than 0.5.0, or one with `api.json_api = false`).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

/// Largest response body accepted from a gateway. Real ones are a few KB; the
/// cap keeps a wrong URL from making us buffer an arbitrary download.
const MAX_BODY_BYTES: usize = 1 << 20;

/// Shown whenever the gateway does not speak the JSON API at all.
const JSON_API_HINT: &str =
    "Landfall needs DATUM Gateway 0.5.0 or newer with `api.json_api` enabled";

// ── wire types ─────────────────────────────────────────────────────
//
// Field names mirror `src/datum_json_api.c` (DATUM Gateway v0.5.1beta). The
// gateway mixes camelCase and snake_case between endpoints; we serialize
// snake_case like every other Landfall payload, and accept the gateway's
// camelCase as an alias, so the same types parse both the gateway's answer
// and our own `/datum/overview`. Every field defaults so a gateway that adds
// or drops one doesn't break the whole tab.

/// `/v1/decentralized_client_stats`: readiness, pool connection and share
/// totals since the gateway started.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct ClientStats {
    #[serde(alias = "acceptedShares")]
    pub accepted_shares: u64,
    #[serde(alias = "acceptedSharesDiff")]
    pub accepted_shares_diff: u64,
    #[serde(alias = "rejectedShares")]
    pub rejected_shares: u64,
    #[serde(alias = "rejectedSharesDiff")]
    pub rejected_shares_diff: u64,
    /// "Ready", "Not Ready", "Non-Pooled Mode and Ready", "Block template
    /// error" or "Waiting for block template to be initialized".
    pub ready: String,
    /// `host:port` of the DATUM pool, or "N/A" when not pooled.
    #[serde(alias = "poolHost")]
    pub pool_host: String,
    #[serde(alias = "poolTag")]
    pub pool_tag: String,
    #[serde(alias = "minerTag")]
    pub miner_tag: String,
    #[serde(alias = "poolMinDiff")]
    pub pool_min_diff: u64,
    #[serde(alias = "poolPubKey")]
    pub pool_pub_key: String,
    /// Seconds since the gateway process started.
    pub uptime: u64,
}

/// `/v1/stratum_server_info`: the Stratum side the miners connect to.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct StratumServerInfo {
    #[serde(alias = "activeThread")]
    pub active_thread: u64,
    #[serde(alias = "totalConnections")]
    pub total_connections: u64,
    #[serde(alias = "totalWorkSubscriptions")]
    pub total_work_subscriptions: u64,
    /// TH/s, estimated from accepted shares.
    #[serde(alias = "estimatedHashrate")]
    pub estimated_hashrate: f64,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct BlockVersion {
    pub int: u64,
    pub hex: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct TemplateTime {
    pub current: u64,
    pub minimum: u64,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct TemplateLimits {
    pub size: u64,
    pub weight: u64,
    pub sigops: u64,
}

/// `/v1/current_stratum_job`: the block template miners are working on.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct StratumJob {
    pub block_height: u64,
    /// Coinbase value (subsidy + fees), in sats.
    pub block_value: u64,
    pub previous_block: String,
    pub block_target: String,
    pub witness_commitment: String,
    pub block_difficulty: f64,
    pub block_version: BlockVersion,
    pub bits: String,
    pub time: TemplateTime,
    pub limits: TemplateLimits,
    pub size: u64,
    pub weight: u64,
    pub sigops: u64,
    pub tx_count: u64,
}

/// One coinbase output of the current job (`/v1/coinbaser`).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct CoinbaseOutput {
    pub address: String,
    pub sats: u64,
}

/// One Stratum thread with live connections (`/v1/thread_stats`).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ThreadStat {
    pub id: u32,
    pub connection_count: u64,
    pub subscription_count: u64,
    /// TH/s.
    pub approx_hashrate: f64,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct RawThreadStat {
    connection_count: u64,
    subscription_count: u64,
    approx_hashrate: f64,
}

/// Everything the DATUM tab renders, fetched in one round.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DatumOverview {
    /// The gateway this snapshot came from.
    pub url: String,
    pub stats: ClientStats,
    pub server: StratumServerInfo,
    /// `None` until the gateway has its first block template.
    pub job: Option<StratumJob>,
    /// Largest output first.
    pub coinbase: Vec<CoinbaseOutput>,
    /// By thread id.
    pub threads: Vec<ThreadStat>,
}

// ── client ─────────────────────────────────────────────────────────

/// Pooled client for DATUM Gateway JSON API calls. The gateway URL is passed
/// per call (it comes from settings), so one client serves every request.
/// Cheap to clone.
#[derive(Debug, Clone)]
pub struct DatumClient {
    http: reqwest::Client,
}

impl Default for DatumClient {
    fn default() -> Self {
        // The gateway is often on the LAN, but may be a VPS behind a VPN, so
        // allow a slow link without letting a dead one hang the tab. No
        // redirects: the stored URL is the only host we ever talk to.
        let http = reqwest::Client::builder()
            .user_agent(concat!("landfall/", env!("CARGO_PKG_VERSION")))
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(10))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .expect("reqwest builder defaults always build");
        Self { http }
    }
}

impl DatumClient {
    /// GET `path` from the gateway at `base` (already normalized, no trailing
    /// slash) and return the JSON body. Error messages name the gateway and
    /// the path but never echo the body, so a mistyped URL can't be used to
    /// read some other service through us.
    async fn get_json(&self, base: &str, path: &str) -> Result<serde_json::Value> {
        let url = format!("{base}{path}");
        let mut resp = self.http.get(&url).send().await.map_err(|e| {
            let why = if e.is_timeout() {
                "timed out"
            } else if e.is_connect() {
                "connection failed"
            } else {
                "request failed"
            };
            Error::Datum(format!(
                "could not reach the gateway at {base} ({why}). Is it running with \
                 `api.listen_port` set in its config?"
            ))
        })?;
        let status = resp.status();
        let body = read_capped(&mut resp, base).await?;
        if status == reqwest::StatusCode::NOT_FOUND {
            return Err(Error::Datum(format!(
                "{base} has no {path} (HTTP 404). {JSON_API_HINT}."
            )));
        }
        if !status.is_success() {
            return Err(Error::Datum(format!(
                "{base} answered {path} with HTTP {status}"
            )));
        }
        serde_json::from_slice(&body).map_err(|_| {
            Error::Datum(format!(
                "{base} did not answer {path} with JSON. Is this the gateway's API port \
                 (`api.listen_port`), not its Stratum port? {JSON_API_HINT}."
            ))
        })
    }

    /// Like [`Self::get_json`], then turn a `{"error": "…"}` body into an
    /// error and decode the rest as `T`.
    async fn get<T: for<'de> Deserialize<'de>>(&self, base: &str, path: &str) -> Result<T> {
        let value = self.get_json(base, path).await?;
        if let Some(msg) = error_message(&value) {
            return Err(Error::Datum(format!("{base}{path}: {msg}")));
        }
        serde_json::from_value(value)
            .map_err(|e| Error::Datum(format!("{base} sent an unexpected {path} payload: {e}")))
    }

    /// `/v1/decentralized_client_stats`
    pub async fn client_stats(&self, base: &str) -> Result<ClientStats> {
        self.get(base, "/v1/decentralized_client_stats").await
    }

    /// `/v1/stratum_server_info`
    pub async fn server_info(&self, base: &str) -> Result<StratumServerInfo> {
        self.get(base, "/v1/stratum_server_info").await
    }

    /// `/v1/current_stratum_job`. `None` while the gateway has no template
    /// yet: it then answers `{"error": "No stratum job"}` with HTTP 200.
    pub async fn current_job(&self, base: &str) -> Result<Option<StratumJob>> {
        let path = "/v1/current_stratum_job";
        let value = self.get_json(base, path).await?;
        if error_message(&value).is_some() {
            return Ok(None);
        }
        serde_json::from_value(value)
            .map(Some)
            .map_err(|e| Error::Datum(format!("{base} sent an unexpected {path} payload: {e}")))
    }

    /// `/v1/coinbaser`: `{ "<address>": <sats>, … }`, largest output first.
    pub async fn coinbaser(&self, base: &str) -> Result<Vec<CoinbaseOutput>> {
        let raw: BTreeMap<String, u64> = self.get(base, "/v1/coinbaser").await?;
        let mut outputs: Vec<CoinbaseOutput> = raw
            .into_iter()
            .map(|(address, sats)| CoinbaseOutput { address, sats })
            .collect();
        outputs.sort_by(|a, b| b.sats.cmp(&a.sats).then_with(|| a.address.cmp(&b.address)));
        Ok(outputs)
    }

    /// `/v1/thread_stats`: `{ "<thread id>": {…}, … }`, by thread id.
    pub async fn thread_stats(&self, base: &str) -> Result<Vec<ThreadStat>> {
        let raw: BTreeMap<String, RawThreadStat> = self.get(base, "/v1/thread_stats").await?;
        let mut threads: Vec<ThreadStat> = raw
            .into_iter()
            .filter_map(|(id, t)| {
                Some(ThreadStat {
                    id: id.parse().ok()?,
                    connection_count: t.connection_count,
                    subscription_count: t.subscription_count,
                    approx_hashrate: t.approx_hashrate,
                })
            })
            .collect();
        threads.sort_by_key(|t| t.id);
        Ok(threads)
    }

    /// Every public endpoint, fetched concurrently. Fails as a whole if any
    /// call fails: a half-rendered tab would hide that the gateway is
    /// unhealthy.
    pub async fn overview(&self, base: &str) -> Result<DatumOverview> {
        let (stats, server, job, coinbase, threads) = tokio::try_join!(
            self.client_stats(base),
            self.server_info(base),
            self.current_job(base),
            self.coinbaser(base),
            self.thread_stats(base),
        )?;
        Ok(DatumOverview {
            url: base.to_string(),
            stats,
            server,
            job,
            coinbase,
            threads,
        })
    }
}

/// The message of a `{"error": "…"}` body, if that's what `value` is.
fn error_message(value: &serde_json::Value) -> Option<&str> {
    value.as_object()?.get("error")?.as_str()
}

/// Read the response body, refusing anything over [`MAX_BODY_BYTES`].
async fn read_capped(resp: &mut reqwest::Response, base: &str) -> Result<Vec<u8>> {
    let too_large = || Error::Datum(format!("{base} sent a response over 1 MiB"));
    if resp
        .content_length()
        .is_some_and(|n| n > MAX_BODY_BYTES as u64)
    {
        return Err(too_large());
    }
    let mut body = Vec::new();
    while let Some(chunk) = resp
        .chunk()
        .await
        .map_err(|_| Error::Datum(format!("lost the connection to {base} mid-response")))?
    {
        if body.len() + chunk.len() > MAX_BODY_BYTES {
            return Err(too_large());
        }
        body.extend_from_slice(&chunk);
    }
    Ok(body)
}

// ── gateway URL ────────────────────────────────────────────────────

/// Validate and normalize a gateway URL as typed by the user.
///
/// A missing scheme means `http://` (DATUM serves plain HTTP; `https://` is
/// for a TLS reverse proxy in front of it). A path prefix is kept for the same
/// reason. Credentials, a query or a fragment are refused: the API takes none
/// of them, and credentials in a stored URL would end up in logs. The result
/// has no trailing slash, so endpoint paths append directly.
pub fn normalize_gateway_url(raw: &str) -> Result<String> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Err(Error::InvalidDatumUrl(
            "enter the gateway's API address, e.g. http://192.168.1.10:7152".into(),
        ));
    }
    let with_scheme = if raw.contains("://") {
        raw.to_string()
    } else {
        format!("http://{raw}")
    };
    let url = reqwest::Url::parse(&with_scheme)
        .map_err(|e| Error::InvalidDatumUrl(format!("{raw}: {e}")))?;
    if !matches!(url.scheme(), "http" | "https") {
        return Err(Error::InvalidDatumUrl(format!(
            "{raw}: only http:// and https:// are supported"
        )));
    }
    if url.host_str().is_none_or(str::is_empty) {
        return Err(Error::InvalidDatumUrl(format!("{raw}: missing a host")));
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err(Error::InvalidDatumUrl(format!(
            "{raw}: remove the user name and password from the URL"
        )));
    }
    if url.query().is_some() || url.fragment().is_some() {
        return Err(Error::InvalidDatumUrl(format!(
            "{raw}: remove the ?query or #fragment"
        )));
    }
    Ok(url.as_str().trim_end_matches('/').to_string())
}

// ── settings ───────────────────────────────────────────────────────

/// What Landfall remembers about the gateway. Persisted as JSON, `0600`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
pub struct DatumSettings {
    /// Normalized gateway URL ([`normalize_gateway_url`]). `None` = no
    /// gateway, and the DATUM tab stays hidden.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

/// Load the settings at `path`. A missing file is "no gateway configured".
/// A stored URL is re-validated, so a hand-edited file can't point the
/// client somewhere [`normalize_gateway_url`] would refuse.
pub fn load_settings(path: &Path) -> Result<DatumSettings> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(DatumSettings::default()),
        Err(e) => return Err(Error::Io(e)),
    };
    let mut settings: DatumSettings = serde_json::from_str(&text).map_err(|e| {
        Error::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("cannot parse {}: {e}", path.display()),
        ))
    })?;
    settings.url = match settings.url.as_deref().map(str::trim) {
        None | Some("") => None,
        Some(url) => Some(normalize_gateway_url(url)?),
    };
    Ok(settings)
}

/// Write `settings` to `path` as a `0600` file, atomically (temp file in the
/// same directory, then rename), so a crash or a concurrent reader never sees
/// a half-written file. Creates the parent directory `0700` if it is missing.
pub fn save_settings(path: &Path, settings: &DatumSettings) -> Result<()> {
    use std::io::Write;

    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        if !parent.exists() {
            std::fs::create_dir_all(parent)?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o700))?;
            }
        }
    }

    let tmp = temp_path(path);
    let write = || -> Result<()> {
        let mut opts = std::fs::OpenOptions::new();
        opts.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            opts.mode(0o600);
        }
        let mut f = opts.open(&tmp)?;
        let mut json = serde_json::to_string_pretty(settings)?;
        json.push('\n');
        f.write_all(json.as_bytes())?;
        f.sync_all()?;
        std::fs::rename(&tmp, path)?;
        Ok(())
    };
    write().inspect_err(|_| {
        let _ = std::fs::remove_file(&tmp);
    })
}

/// A temp file name next to `path`, unique per process and call, so two
/// concurrent saves never write into the same temp file.
fn temp_path(path: &Path) -> PathBuf {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "datum.json".into());
    path.with_file_name(format!(
        ".{name}.{}.{}.tmp",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    use axum::routing::get;
    use axum::Router;

    const CLIENT_STATS: &str = r#"{"acceptedShares":1234,"acceptedSharesDiff":5678900,
        "rejectedShares":3,"rejectedSharesDiff":12000,"ready":"Ready",
        "poolHost":"datum-beta1.mine.ocean.xyz:28915","poolTag":"OCEAN.XYZ",
        "minerTag":"DATUM User","poolMinDiff":16384,"poolPubKey":"f21f2f0e","uptime":86400}"#;
    const SERVER_INFO: &str = r#"{"activeThread":2,"totalConnections":3,
        "totalWorkSubscriptions":3,"estimatedHashrate":215.37}"#;
    const JOB: &str = r#"{"block_height":921337,"block_value":313512345,
        "previous_block":"00000000000000000001a2b3","block_target":"0000000000000000000234",
        "witness_commitment":"6a24aa21a9ed","block_difficulty":146472570619930.8,
        "block_version":{"int":536870912,"hex":"20000000"},"bits":"17023a04",
        "time":{"current":1761900000,"minimum":1761897000},
        "limits":{"size":4000000,"weight":4000000,"sigops":80000},
        "size":1543210,"weight":3993210,"sigops":12345,"tx_count":3456}"#;
    const COINBASER: &str = r#"{"bc1qsmall":1000,"bc1qbig":312511345}"#;
    const THREADS: &str = r#"{"10":{"connection_count":1,"subscription_count":1,"approx_hashrate":65.2},
        "2":{"connection_count":2,"subscription_count":2,"approx_hashrate":150.1}}"#;

    fn json_route(body: &'static str) -> axum::routing::MethodRouter {
        get(move || async move { ([("content-type", "application/json")], body) })
    }

    /// A gateway that answers every public endpoint, with `job` as the
    /// `/v1/current_stratum_job` body.
    fn gateway(job: &'static str) -> Router {
        Router::new()
            .route("/v1/decentralized_client_stats", json_route(CLIENT_STATS))
            .route("/v1/stratum_server_info", json_route(SERVER_INFO))
            .route("/v1/current_stratum_job", json_route(job))
            .route("/v1/coinbaser", json_route(COINBASER))
            .route("/v1/thread_stats", json_route(THREADS))
    }

    async fn serve(app: Router) -> String {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            let _ = axum::serve(listener, app).await;
        });
        format!("http://{addr}")
    }

    fn datum_err(r: Result<impl std::fmt::Debug>) -> String {
        match r {
            Err(Error::Datum(msg)) => msg,
            other => panic!("expected Error::Datum, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn overview_decodes_every_public_endpoint() {
        let base = serve(gateway(JOB)).await;
        let ov = DatumClient::default().overview(&base).await.unwrap();

        assert_eq!(ov.url, base);
        assert_eq!(ov.stats.ready, "Ready");
        assert_eq!(ov.stats.accepted_shares, 1234);
        assert_eq!(ov.stats.rejected_shares_diff, 12000);
        assert_eq!(ov.stats.pool_host, "datum-beta1.mine.ocean.xyz:28915");
        assert_eq!(ov.stats.pool_pub_key, "f21f2f0e");
        assert_eq!(ov.stats.uptime, 86400);
        assert_eq!(ov.server.total_work_subscriptions, 3);
        assert!((ov.server.estimated_hashrate - 215.37).abs() < 1e-9);

        let job = ov.job.expect("a job");
        assert_eq!(job.block_height, 921337);
        assert_eq!(job.block_value, 313512345);
        assert_eq!(job.block_version.hex, "20000000");
        assert_eq!(job.limits.weight, 4_000_000);
        assert_eq!(job.tx_count, 3456);

        // Largest output first.
        assert_eq!(
            ov.coinbase,
            vec![
                CoinbaseOutput {
                    address: "bc1qbig".into(),
                    sats: 312511345
                },
                CoinbaseOutput {
                    address: "bc1qsmall".into(),
                    sats: 1000
                },
            ]
        );
        // Numeric thread order, not the map's string order ("10" < "2").
        let ids: Vec<u32> = ov.threads.iter().map(|t| t.id).collect();
        assert_eq!(ids, vec![2, 10]);
    }

    #[tokio::test]
    async fn overview_serializes_snake_case() {
        let base = serve(gateway(JOB)).await;
        let ov = DatumClient::default().overview(&base).await.unwrap();
        let v = serde_json::to_value(&ov).unwrap();
        assert_eq!(v["stats"]["accepted_shares"], 1234);
        assert_eq!(v["stats"]["pool_pub_key"], "f21f2f0e");
        assert_eq!(v["server"]["total_work_subscriptions"], 3);
        assert!(v["stats"].get("acceptedShares").is_none());

        // And our own snake_case output parses back without losing fields
        // (what a client of `/datum/overview` does), not into silent defaults.
        let back: DatumOverview = serde_json::from_value(v).unwrap();
        assert_eq!(back.stats.accepted_shares, 1234);
        assert_eq!(back.stats.pool_host, "datum-beta1.mine.ocean.xyz:28915");
        assert_eq!(back.server.total_connections, 3);
        assert_eq!(back.job.unwrap().tx_count, 3456);
    }

    #[tokio::test]
    async fn no_template_yet_is_a_missing_job_not_an_error() {
        let base = serve(gateway(r#"{"error":"No stratum job"}"#)).await;
        let ov = DatumClient::default().overview(&base).await.unwrap();
        assert!(ov.job.is_none());
        assert_eq!(ov.stats.ready, "Ready");
    }

    #[tokio::test]
    async fn gateway_without_json_api_names_the_version_requirement() {
        // DATUM < 0.5.0 (or `api.json_api = false`) answers `/v1/*` with an
        // HTML 404.
        let app = Router::new()
            .fallback(|| async { (axum::http::StatusCode::NOT_FOUND, "<H1>Not found</H1>") });
        let base = serve(app).await;
        let msg = datum_err(DatumClient::default().overview(&base).await);
        assert!(msg.contains("HTTP 404"), "{msg}");
        assert!(msg.contains("0.5.0"), "{msg}");
    }

    #[tokio::test]
    async fn non_json_answer_points_at_the_api_port_and_hides_the_body() {
        let app = Router::new().fallback(|| async { "SECRET-INTERNAL-PAGE" });
        let base = serve(app).await;
        let msg = datum_err(DatumClient::default().client_stats(&base).await);
        assert!(msg.contains("api.listen_port"), "{msg}");
        assert!(!msg.contains("SECRET-INTERNAL-PAGE"), "body leaked: {msg}");
    }

    #[tokio::test]
    async fn error_envelope_becomes_an_error() {
        let app = Router::new().route(
            "/v1/decentralized_client_stats",
            json_route(r#"{"error":"boom"}"#),
        );
        let base = serve(app).await;
        let msg = datum_err(DatumClient::default().client_stats(&base).await);
        assert!(
            msg.ends_with("/v1/decentralized_client_stats: boom"),
            "{msg}"
        );
    }

    #[tokio::test]
    async fn unreachable_gateway_says_so() {
        // Bind, then free the port: nothing listens there any more.
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        drop(listener);
        let msg = datum_err(DatumClient::default().client_stats(&base).await);
        assert!(msg.contains("could not reach the gateway"), "{msg}");
        assert!(msg.contains("api.listen_port"), "{msg}");
    }

    #[tokio::test]
    async fn oversized_body_is_refused() {
        let app = Router::new().route(
            "/v1/decentralized_client_stats",
            get(|| async { "x".repeat(MAX_BODY_BYTES + 1) }),
        );
        let base = serve(app).await;
        let msg = datum_err(DatumClient::default().client_stats(&base).await);
        assert!(msg.contains("over 1 MiB"), "{msg}");
    }

    #[tokio::test]
    async fn redirects_are_not_followed() {
        let target = serve(gateway(JOB)).await;
        let location = format!("{target}/v1/decentralized_client_stats");
        let app = Router::new().route(
            "/v1/decentralized_client_stats",
            get(move || {
                let location = location.clone();
                async move {
                    (
                        axum::http::StatusCode::FOUND,
                        [(axum::http::header::LOCATION, location)],
                    )
                }
            }),
        );
        let base = serve(app).await;
        let msg = datum_err(DatumClient::default().client_stats(&base).await);
        assert!(msg.contains("HTTP 302"), "{msg}");
    }

    #[test]
    fn url_normalization_accepts_what_a_miner_types() {
        for (raw, want) in [
            ("http://192.168.1.10:7152", "http://192.168.1.10:7152"),
            ("http://192.168.1.10:7152/", "http://192.168.1.10:7152"),
            ("  192.168.1.10:7152  ", "http://192.168.1.10:7152"),
            ("umbrel.local:7152", "http://umbrel.local:7152"),
            ("HTTP://Umbrel.Local:7152", "http://umbrel.local:7152"),
            (
                "https://datum.example.com/gateway/",
                "https://datum.example.com/gateway",
            ),
            ("http://[fd00::10]:7152", "http://[fd00::10]:7152"),
        ] {
            assert_eq!(normalize_gateway_url(raw).unwrap(), want, "{raw}");
        }
    }

    #[test]
    fn url_normalization_refuses_what_the_api_cannot_use() {
        for raw in [
            "",
            "   ",
            "ftp://192.168.1.10:7152",
            "file:///etc/passwd",
            "http://admin:pw@192.168.1.10:7152",
            "http://admin@192.168.1.10:7152",
            "http://192.168.1.10:7152/?x=1",
            "http://192.168.1.10:7152/#top",
            "http://",
        ] {
            assert!(
                matches!(normalize_gateway_url(raw), Err(Error::InvalidDatumUrl(_))),
                "{raw:?} should be refused"
            );
        }
    }

    fn settings_path(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("landfall-datum-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir.join("datum.json")
    }

    #[test]
    fn missing_settings_file_means_not_configured() {
        let path = settings_path("missing");
        assert_eq!(load_settings(&path).unwrap(), DatumSettings::default());
    }

    #[test]
    fn settings_round_trip_owner_only() {
        let path = settings_path("roundtrip");
        let settings = DatumSettings {
            url: Some("http://192.168.1.10:7152".into()),
        };
        save_settings(&path, &settings).unwrap();
        assert_eq!(load_settings(&path).unwrap(), settings);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
            assert_eq!(mode, 0o600);
            let dir = std::fs::metadata(path.parent().unwrap()).unwrap();
            assert_eq!(dir.permissions().mode() & 0o777, 0o700);
        }
        // Clearing writes an empty object, which loads as "no gateway".
        save_settings(&path, &DatumSettings::default()).unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap().trim(), "{}");
        assert_eq!(load_settings(&path).unwrap().url, None);
        // No temp files left behind.
        let leftovers = std::fs::read_dir(path.parent().unwrap()).unwrap().count();
        assert_eq!(leftovers, 1);
    }

    #[test]
    fn hand_edited_settings_are_revalidated() {
        let path = settings_path("handedited");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, r#"{"url":"umbrel.local:7152/"}"#).unwrap();
        assert_eq!(
            load_settings(&path).unwrap().url.as_deref(),
            Some("http://umbrel.local:7152")
        );
        std::fs::write(&path, r#"{"url":"file:///etc/passwd"}"#).unwrap();
        assert!(matches!(
            load_settings(&path),
            Err(Error::InvalidDatumUrl(_))
        ));
        std::fs::write(&path, "not json").unwrap();
        assert!(matches!(load_settings(&path), Err(Error::Io(_))));
    }
}
