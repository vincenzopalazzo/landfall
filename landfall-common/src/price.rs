//! Live BTC/USD spot price.
//!
//! Rust twin of [`landfall-web/src/lib/price.ts`] — same endpoint, same 60s
//! in-process cache, same failure posture. It exists here for the same reason
//! [`crate::ocean`] does: Rust callers (the mobile core today, httpd/MCP
//! tomorrow) should get the price the same way the frontend does instead of
//! each inventing a rate.
//!
//! **A missing price is never a fabricated price.** [`PriceClient::btc_usd`]
//! returns `0.0` when the value is unavailable (offline, blocked, or a bad
//! response) and callers treat `0` as "USD not available" and fall back to
//! sats. The last good value survives a flaky fetch so the display doesn't
//! flicker, but only up to [`MAX_STALE`] — an hours-old rate shown as current
//! next to a wallet balance is exactly the fabricated figure this avoids.

use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde::Deserialize;

/// mempool.space's public price endpoint: CORS-open, no API key, and already
/// a crypto-native dependency we link to for block/tx explorers.
pub const PRICE_URL: &str = "https://mempool.space/api/v1/prices";

/// How long a fetched price stays fresh. Matches `price.ts`'s `TTL_MS`.
const TTL: Duration = Duration::from_secs(60);

/// How long a *stale* price may still be served when refreshes are failing.
///
/// Past this, `btc_usd` reports unavailable rather than a price from an
/// unknown time ago. The frontend's `price.ts` keeps the last good value
/// indefinitely, which is fine for a dashboard; here the figure sits under a
/// wallet balance and on the send-review screen, and BTC can move a long way
/// in an hour offline. Showing sats beats showing a confident wrong number.
const MAX_STALE: Duration = Duration::from_secs(15 * 60);

#[derive(Debug, Deserialize)]
struct Prices {
    #[serde(rename = "USD")]
    usd: Option<f64>,
}

#[derive(Debug, Default)]
struct Cache {
    value: f64,
    fetched_at: Option<Instant>,
}

impl Cache {
    fn fresh(&self) -> bool {
        self.value > 0.0 && self.age().is_some_and(|age| age < TTL)
    }

    /// Still worth serving while refreshes fail — recent enough to be honest.
    fn servable(&self) -> bool {
        self.value > 0.0 && self.age().is_some_and(|age| age < MAX_STALE)
    }

    fn age(&self) -> Option<Duration> {
        self.fetched_at.map(|at| at.elapsed())
    }
}

/// Pooled BTC/USD price client. Construct one per process and reuse it — the
/// cache lives on the instance, so a shared client is what makes the 60s TTL
/// meaningful.
#[derive(Debug)]
pub struct PriceClient {
    http: reqwest::Client,
    url: String,
    cache: Mutex<Cache>,
}

impl Default for PriceClient {
    fn default() -> Self {
        // 10s timeout: the price is decoration next to a balance, so it must
        // never be what makes a screen hang. On timeout we keep the last good
        // value and the caller falls back to sats.
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .expect("reqwest builder defaults always build");
        Self {
            http,
            url: PRICE_URL.to_string(),
            cache: Mutex::new(Cache::default()),
        }
    }
}

impl PriceClient {
    /// Point at a custom URL (tests use a local mock).
    pub fn with_url(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            ..Self::default()
        }
    }

    /// Current BTC/USD spot, or `0.0` if unavailable.
    ///
    /// Never returns an error: a price we could not fetch is not an error
    /// condition for the caller, it is a reason to show sats. Cached for
    /// [`TTL`]. A failed refresh keeps serving the previous value only while
    /// it is under [`MAX_STALE`] old — past that the caller is told the price
    /// is unavailable rather than handed one from an unknown time ago.
    pub async fn btc_usd(&self) -> f64 {
        // Fast path: still fresh. The lock is never held across an await.
        if let Ok(cache) = self.cache.lock() {
            if cache.fresh() {
                return cache.value;
            }
        }

        let fetched = self.fetch().await;
        let mut cache = match self.cache.lock() {
            Ok(c) => c,
            // A poisoned lock means another thread panicked mid-update. The
            // price is not worth propagating that panic — report unavailable.
            Err(_) => return 0.0,
        };
        if let Some(usd) = fetched {
            cache.value = usd;
            cache.fetched_at = Some(Instant::now());
            return cache.value;
        }
        // Refresh failed. Serve the last good value only while it is recent
        // enough to still mean something.
        if cache.servable() {
            cache.value
        } else {
            0.0
        }
    }

    /// One request; `None` on any failure (network, non-2xx, bad JSON, or a
    /// non-positive/non-finite value).
    async fn fetch(&self) -> Option<f64> {
        let resp = self.http.get(&self.url).send().await.ok()?;
        if !resp.status().is_success() {
            return None;
        }
        let prices: Prices = resp.json().await.ok()?;
        let usd = prices.usd?;
        (usd.is_finite() && usd > 0.0).then_some(usd)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_is_stale_when_never_fetched() {
        let cache = Cache::default();
        assert!(!cache.fresh());
    }

    #[test]
    fn cache_is_stale_when_value_is_zero() {
        // A zero value must never count as fresh, or one failed fetch would
        // pin "USD unavailable" for the whole TTL.
        let cache = Cache {
            value: 0.0,
            fetched_at: Some(Instant::now()),
        };
        assert!(!cache.fresh());
    }

    #[test]
    fn cache_is_fresh_right_after_a_good_fetch() {
        let cache = Cache {
            value: 96_200.0,
            fetched_at: Some(Instant::now()),
        };
        assert!(cache.fresh());
    }

    #[test]
    fn a_recently_stale_price_is_still_servable() {
        // Refreshes failing for a minute should not blank the fiat display.
        let cache = Cache {
            value: 96_200.0,
            fetched_at: Some(Instant::now()),
        };
        assert!(cache.servable());
        assert!(cache.fresh());
    }

    #[test]
    fn a_zero_value_is_never_servable() {
        let cache = Cache {
            value: 0.0,
            fetched_at: Some(Instant::now()),
        };
        assert!(!cache.servable());
    }

    #[test]
    fn a_never_fetched_cache_is_not_servable() {
        assert!(!Cache::default().servable());
    }

    #[test]
    fn max_stale_is_longer_than_the_freshness_window() {
        // Otherwise a value would go unavailable the moment it went stale,
        // and a single flaky request would blank the display.
        assert!(MAX_STALE > TTL);
    }

    #[test]
    fn prices_body_parses() {
        let p: Prices = serde_json::from_str(r#"{"USD":96200.5,"EUR":88000}"#).unwrap();
        assert_eq!(p.usd, Some(96200.5));
    }

    #[test]
    fn prices_body_tolerates_missing_usd() {
        let p: Prices = serde_json::from_str(r#"{"EUR":88000}"#).unwrap();
        assert_eq!(p.usd, None);
    }
}
