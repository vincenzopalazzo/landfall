# Brainstorm: read-only MCP server on `oceanln-httpd`

## Clarified Problem Statement

**Goal:** Ship a read-only MCP server that lets Claude Code (or any MCP client)
query the user's OCEAN payout state — wallet status, Lightning payouts, public
OCEAN mining stats — without any ability to sign, spend, or otherwise execute
wallet operations. Mountable as a `/mcp` route on `oceanln-httpd` so the
existing Docker / Umbrel / Start9 deployments get MCP for free.

**Constraints:**
- HTTP transport only (Streamable HTTP per the rmcp 1.7.0 SDK). No stdio.
- **Strictly read-only.** No re-sign, no offer creation, no payout, no
  seed-touching writes. Per the directive: "check everything for them, but not
  execute, because it is dangerous."
- Mount under the existing bearer-auth — same token as the rest of
  `oceanln-httpd`, no new auth model.
- Single `ServerHandler` implementation; HTTP transport is a thin binding —
  mirrors the PR-A pattern (one Rust core, two transports).

**Non-goals:**
- stdio support (deferred — would unblock Claude Desktop / Claude Code direct
  subprocess use, but everything we ship later is HTTP-deploy anyway)
- Re-sign / create-offer / pay-invoice tools (deferred — needs a per-call
  consent UX before it's safe to expose to an AI)
- Per-tool fine-grained permissions / OS keychain (deferred)
- MCP integration in the Tauri desktop app (deferred — desktop has no HTTP
  server by design; users wanting MCP run `oceanln-httpd` / Docker)
- Multi-tenant / multi-user concerns (single-user container, same as the rest
  of the deploy story)

**Success criteria:**
- An MCP client (e.g. `claude mcp add` with the HTTP add-syntax) can connect
  to `http://127.0.0.1:5393/mcp` against a running `oceanln-httpd` with
  bearer auth and discover the 7 tools.
- Claude can ask "list my recent OCEAN payouts" and the AI gets the same
  `Vec<OceanPayout>` the dashboard renders.
- Claude can ask "what's my current hashrate?" and the AI fetches
  `api.ocean.xyz/v1/user_hashrate` via the MCP tool (the user's MCP client
  typically can't reach the public internet directly).
- **Zero tools that touch a private key.**
- The dashboard MCP panel stops saying "coming soon" and shows the real
  endpoint URL + add command.

## Recommended Approach: `/mcp` route on `oceanln-httpd`, new `oceanln-mcp` crate

- **Sketch:** New crate `oceanln-mcp` defines `OceanMcpService` implementing
  `rmcp::ServerHandler`. Tool handlers call straight into
  `oceanln-common::lexe_wallet::list_offer_payouts`,
  `oceanln-httpd::service::status`, and a small wrapper around the OCEAN
  public-API client. `oceanln-httpd` mounts it at `GET/POST /mcp` using
  `rmcp::transport::streamable_http::StreamableHttpService`. Same bearer-auth
  as the rest of the routes.

- **Affected files / new files:**
  - New crate: `oceanln-mcp/{Cargo.toml, src/lib.rs, src/tools/{status,payouts,ocean_stats,health}.rs}`
  - `oceanln-httpd/src/lib.rs` — mount `/mcp` under the existing `protected` router
  - `oceanln-httpd/Cargo.toml` — add `oceanln-mcp` path dep + `rmcp` HTTP-server features
  - Workspace `Cargo.toml` — add `rmcp = { version = "1.7", features = ["server", "transport-streamable-http-server", "macros"] }`
  - `oceanln-web/src/lib/data.ts` — update MCP panel content (drop "coming soon", switch to the HTTP add command)
  - `oceanln-web/src/lib/Dashboard.svelte` — show the actual endpoint URL (`{base}/mcp`) + correct connect command

- **Tool surface (all read-only):**
  1. `get_status` — `oceanln_httpd::service::status` (configured / mining_address / offer)
  2. `list_payouts` — `oceanln_common::lexe_wallet::list_offer_payouts` (PR-A path)
  3. `get_ocean_stats` — proxies `api.ocean.xyz/v1/statsnap/<addr>`
  4. `get_ocean_hashrate` — proxies `/v1/user_hashrate/<addr>`
  5. `get_ocean_earnings` — proxies `/v1/earnpay/<addr>`
  6. `get_ocean_pool_stats` — proxies `/v1/pool_stat`
  7. `get_health` — Lexe wallet health

- **Tradeoffs:** Reuses the existing httpd auth/CORS/listening port — zero
  new ops surface. Desktop app doesn't get MCP (acceptable; desktop has no
  HTTP server by design — document "run oceanln-httpd for MCP"). Forces dep
  on rmcp's HTTP feature flags into httpd's compile graph.

- **Effort:** M (~1.5 days)

## Open questions (non-blocking)

- **MCP-over-HTTP add-command syntax:** `claude mcp add` currently expects a
  stdio binary. The streamable-HTTP add syntax has shifted in recent Claude
  Code releases. Worth confirming the exact `claude mcp add` invocation
  against the current CLI before promising it in the UI.
- **Browser-served MCP:** Since httpd serves the SPA from the same origin
  (per the PR B plan), should the SPA itself expose an in-browser
  MCP-over-WebSocket bridge so a hosted webapp can drive its own MCP
  integration? Probably out of scope; flag it.
- **Rate-limiting / abuse:** an AI in a loop could spam OCEAN's public API
  through our proxy. A simple per-tool rate limit (e.g. 1 req/s per tool)
  is worth adding before the Umbrel deploy.
