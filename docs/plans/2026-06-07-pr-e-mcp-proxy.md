# Plan: PR E — refactor MCP into a standalone proxy binary

**Goal:** Recast `oceanln-mcp` so it is **a thin protocol adapter** in front of
`oceanln-httpd`'s REST API, not a parallel code path into the wallet. Tool
handlers become reqwest calls to `http://<httpd>/status`,
`/payouts`, `/ocean/statsnap/<addr>`, etc. The invariant becomes simple
to audit: **anything an AI can do via MCP is exactly what a human can do
via REST**.

This corrects an architectural call from PR D that the user has now
overruled: "the mcp server should be just a proxy for AI to call what
it is already possible to call in the httpd server."

## What changes

**`oceanln-mcp` becomes a runnable binary.**
- New `src/main.rs` with clap CLI: `--base <url>`, `--bind <addr>`,
  optional `--token <T>`, `--httpd-token <T>`.
- Starts its own `StreamableHttpService` listener on its own port.
- Tool handlers call `reqwest` against `--base` (httpd) — never
  touch `oceanln-common::service` or `wallet_provider` again.
- Drops the `--features = lexe-sdk` line on its dep on
  `oceanln-common`; it now only needs the response *types*
  (`StatusResp`, `OceanPayout`, `StatSnap`, etc.) for typed
  deserialization.

**`oceanln-httpd` gains 4 OCEAN-proxy endpoints**, since the four
public-API tools (`get_ocean_stats` / `get_ocean_hashrate` /
`get_ocean_earnings` / `get_ocean_pool_stats`) MUST be reachable
via REST first if MCP only proxies REST:
- `GET /ocean/statsnap/<addr>`
- `GET /ocean/earnpay/<addr>`
- `GET /ocean/user_hashrate/<addr>`
- `GET /ocean/pool_stat`

Each is a 5-line handler calling `OceanClient::*`. Side benefit: the
web frontend can later swap its direct `api.ocean.xyz` calls for these
to centralize CORS, caching, and rate limiting.

**`oceanln-httpd` `--token` becomes optional.** Per the new constraint
that v1 isn't exposed to the public network, the loopback bind is the
sole defense. When `--token` is omitted, the `guard` middleware does
NOT require an `Authorization` header (origin + host checks still
apply). When `--token` is set, the current strict behavior is
preserved.

**`/mcp` route on `oceanln-httpd` is REMOVED.**
- Drops the `rmcp` workspace dep from `oceanln-httpd`.
- Drops `oceanln-httpd`'s path dep on `oceanln-mcp`.
- Removes `mcp_endpoint_*` integration tests.
- Removes the auth-bypass concern entirely (the route is just gone).

**Dashboard MCP panel** updates `addCmd` to reflect the new flow:
the user starts two processes — `oceanln-httpd` and `oceanln-mcp` —
and points their MCP client at the standalone binary's port.

## Architectural invariant

```
                    ┌─── oceanln-mcp process ───┐
                    │   StreamableHttpService   │
MCP client (Goose,  │   ┌─────────────────┐     │
Claude Code, …) ────┼──▶│  #[tool] fn …   │     │
                    │   └────────┬────────┘     │
                    │            │              │
                    │            │ reqwest      │
                    │            ▼              │
                    └────────────│──────────────┘
                                 │ HTTP (loopback)
                                 ▼
                    ┌─── oceanln-httpd process ───┐
                    │   /status                   │
                    │   /payouts                  │
                    │   /ocean/statsnap/<addr>    │
                    │   /ocean/earnpay/<addr>     │
                    │   /ocean/user_hashrate/…    │
                    │   /ocean/pool_stat          │
                    │       │                     │
                    │       ▼                     │
                    │  oceanln-common::service    │
                    │  oceanln-common::ocean      │
                    └─────────────────────────────┘
```

If a capability isn't in the REST router, MCP can't expose it. This is
exactly the discipline the user asked for.

## Affected files

**Modified:**
- `oceanln-httpd/src/lib.rs`
  - Add 4 `GET /ocean/*` route handlers
  - Make `guard` middleware skip the bearer check when
    `state.cfg.token` is empty
  - Remove the `/mcp` `nest_service` mount + the rmcp imports
- `oceanln-httpd/src/main.rs` — make `--token` optional
- `oceanln-httpd/Cargo.toml` — drop `rmcp`, drop `oceanln-mcp` dep
- `oceanln-httpd/tests/server.rs`
  - Remove `mcp_endpoint_*` tests
  - Add `/ocean/*` integration tests (returning canned JSON from a
    test OCEAN-mock server)
  - Add a smoke test proving `--token=""` accepts unauthed
- `oceanln-common/src/ocean.rs` — already has `OceanClient`; expose
  the response types as `pub` if any aren't (they all are).
- `oceanln-mcp/Cargo.toml`
  - Add `clap`, `axum`, `tower`, `tokio` features for the binary
  - Add `reqwest` directly (already there)
  - Drop the `lexe-sdk` feature on `oceanln-common`
- `oceanln-mcp/src/lib.rs`
  - Replace `OceanMcpState` fields (`seed`, `wallet`, `ocean`) with a
    single `OceanHttpdClient` (a small reqwest wrapper)
  - Each tool handler becomes `let resp = self.client.foo().await?;
    json_result(&resp)`
  - Drop the per-tool semaphore — rate-limiting moves to httpd's
    `/ocean/*` handlers (or stays absent for v1)
- **NEW** `oceanln-mcp/src/main.rs` — clap CLI + axum router +
  `StreamableHttpService::new(...)` on its own listener
- `oceanln-web/src/lib/data.ts` + `Dashboard.svelte` — update the
  copy-cmd to reflect the standalone binary + new port

**Removed (in oceanln-httpd):**
- `pub use oceanln_mcp::*` and related route mount lines

## Test plan

- Existing oceanln-httpd integration tests still pass (CORS, auth,
  /payouts, /status etc.)
- New oceanln-httpd integration tests:
  - `GET /ocean/pool_stat` returns 200 + a JSON body matching the
    `PoolStat` shape (using a local mock OCEAN server bound to a
    random port + `OceanClient::with_base`)
  - `GET /status` accepts unauthed requests when the server was
    started without `--token`
- oceanln-mcp gets a minimal integration test that starts a
  real `oceanln-httpd` on a random port (no token) + spawns the
  MCP service against it + hits `get_status` through the MCP
  protocol layer, asserting the JSON response makes a round trip
- Drop the obsolete `mcp_endpoint_*` tests in
  `oceanln-httpd/tests/server.rs`

## Conventions
- Same `cargo fmt --check`, `cargo clippy -D warnings` for both
  feature sets (full + thin)
- `mod tests` at end of each file
- All new types `#[derive(Debug, Clone, Serialize, Deserialize)]`

## Open questions / risks
- **Loopback HTTP loop is fine**: a single-host deploy now has
  MCP → httpd over `127.0.0.1`. Adds one TCP round-trip per tool
  call (sub-ms on loopback). Acceptable.
- **No auth in v1**: documented as a v1 constraint. Adding bearer
  back later is a one-line guard change.

## Size: M-L (~300-500 LOC net)
