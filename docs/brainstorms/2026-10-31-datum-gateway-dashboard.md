# Brainstorm: a DATUM tab that makes Landfall the onboarding control for DATUM Gateway

## Clarified Problem Statement

**Goal:** Add a third top-nav surface, **DATUM**, after **Lightning**
(`landfall-web/src/App.svelte`), that connects to a miner's
[DATUM Gateway](https://github.com/OCEAN-xyz/datum_gateway) wherever it runs,
shows its live state and connected workers, and lets the miner configure it.
In particular, it can point the gateway's `mining.pool_address` at the payout
address Landfall derives. Landfall becomes the onboarding and control surface
for DATUM, not just a viewer.

**What the gateway exposes (verified against `src/datum_json_api.c` and
`src/datum_api.c` on `main`, release v0.5.0beta, 2026-10-19):**

| Endpoint | Auth | Data |
|---|---|---|
| `GET /v1/decentralized_client_stats` | none | ready state, pool host, pool/miner tag, min diff, pool pubkey, uptime, accepted/rejected shares |
| `GET /v1/stratum_server_info` | none | active threads, connections, subscriptions, estimated hashrate |
| `GET /v1/current_stratum_job` | none | height, block value, prev hash, target, difficulty, version, size/weight/sigops, tx count |
| `GET /v1/coinbaser` | none | coinbase outputs (address, value) |
| `GET /v1/thread_stats` | none | per-thread connections, subscriptions, hashrate |
| `GET /v1/stratum_client_list` | Digest SHA-256, user `admin` | per-worker remote host, username, vdiff, accepted/rejected, reject %, hashrate, last share, user agent |
| `GET /v1/configuration` | Digest SHA-256 | `pool_address`, `reward_sharing` (require/prefer/never), pool host/port/pubkey, coinbase tags, `rpcurl`/`rpcuser`, `csrf_token` |
| `POST /v1/configuration` | Digest + `X-DATUM-CSRF` + `Content-Type: application/json` + `api.modify_conf=true` | partial config update; many fields trigger a gateway restart (re-exec) |

Facts that shape the design:
- The API is **off by default**: `api.listen_port` defaults to `0`.
  `api.json_api` defaults to `true` but only exists from v0.5.0beta.
  Older gateways answer `/v1/*` with an HTML 404.
- **No CORS headers.** A browser page on another origin cannot read the
  responses, so both builds need a Rust-side client: an httpd proxy route for
  the web build and a Tauri command for desktop. The existing `/ocean/*` routes
  in `landfall-httpd/src/lib.rs` and `landfall-common/src/ocean.rs` show the
  pattern.
- Admin endpoints use **HTTP Digest (SHA-256)**, which `reqwest` does not
  speak natively.
- `GET /v1/configuration` returns the CSRF token that `POST` needs, so writes
  take two requests: read the config for the token, then post.

**Constraints:**
- Works with a gateway **anywhere**: LAN, a remote VPS, behind a VPN. The URL
  is user-configured.
- **Both builds**: browser + `landfall-httpd`, and Tauri desktop. One Rust
  client in `landfall-common` with two thin transports, like OCEAN and Lexe
  today.
- **Hidden until configured**: the DATUM pill appears only after a gateway URL
  is saved, so the setup entry point has to live elsewhere (Settings panel,
  plus a CTA on Done/Profile).
- No open proxy. The httpd proxies **only** to the stored gateway URL, never to
  a URL supplied per request. Setting the URL and password is a token-gated
  `POST`, like `/import`.
- Admin reads (`/clients` exposes miner IPs, `/configuration` exposes
  `rpcuser`) and every write are token-gated in httpd, even in `--no-auth`
  mode. The public stats sit behind the shared `guard`, like `/ocean/*`.
- The admin password is stored like the seed: a plaintext `0600` file next to
  it (Landfall's stated showcase posture). It is never returned to the
  frontend.
- Writes must survive the gateway restart they trigger: poll until it answers
  again, then re-read the config.
- Plain `http://` to a non-local host sends stats and config, CSRF token
  included, in cleartext. Digest keeps the password itself off the wire. Warn
  in the UI and recommend VPN or a TLS reverse proxy.

**Non-goals (v1):**
- Running, installing or upgrading DATUM itself. Landfall connects to an
  existing gateway.
- Several gateways at once.
- Tor `.onion` URLs (needs reqwest SOCKS support; defer).
- MCP tools for DATUM in `landfall-mcp` (easy follow-up once the httpd routes
  exist).
- Mobile (`mobile/`) client.
- The legacy HTML `/cmd` actions (kick worker, empty thread). The JSON API has
  no equivalent.

**Success criteria:**
- With no gateway configured, there is no DATUM pill. After the user saves a
  URL in Settings, the pill appears between Lightning and the account chip.
- Against a v0.5.0+ gateway, the tab shows ready state, uptime, pool host/tag,
  hashrate, shares (accepted/rejected), connections, current job (height,
  value, tx count) and coinbaser outputs, refreshed on an interval.
- With an admin password, a workers table shows each miner's hashrate,
  reject %, last share and user agent.
- The config panel shows the current `pool_address` and flags it when it
  differs from Landfall's payout address. "Use my Landfall address" posts it,
  rides through the restart, and shows the new value.
- Clear states for: unreachable, pre-0.5 gateway (404), `json_api` off,
  admin password not set on the gateway, wrong password, `modify_conf` off.
- The same behaviour in the browser build (via httpd) and desktop (via IPC),
  with vitest + Rust unit tests on the client against recorded fixtures, and a
  QA scenario in `docs/QA-SCENARIOS.md`.

## Approaches Considered

### Approach A: Rust DATUM client, Landfall proxies everything, native UI
- Sketch: add `landfall-common/src/datum.rs` with a `DatumClient` (reqwest
  plus a Digest SHA-256 layer) that has typed structs for all eight calls and
  a `DatumConfig { url, admin_password }` persisted at `0600`. Expose it as
  `landfall-httpd` routes `/datum/{settings,status,job,coinbaser,threads,clients,config}`
  and as matching Tauri commands. On the frontend, add `lib/datum.ts` (the
  same dual httpd/IPC switch as `api.ts`/`tauri.ts`), `lib/Datum.svelte`, a
  Settings section and the nav pill.
- Affected files: `landfall-common/src/{datum.rs,lib.rs,error.rs}`,
  `landfall-httpd/src/lib.rs`, `src-tauri/src/lib.rs`,
  `landfall-web/src/{App.svelte,lib/Settings.svelte,lib/store.svelte.ts,lib/datum.ts,lib/Datum.svelte}`,
  `docs/QA-SCENARIOS.md`, `scripts/qa`.
- Tradeoffs: covers all three tiers in both builds. The secrets and the
  cross-origin problem stay in Rust, where the seed already lives. It costs the
  most surface: a new secret on disk, new token-gated write routes, Digest auth
  to implement and test, and restart handling.
- Effort: L (splits cleanly into three M PRs; see Recommendation).

### Approach B: Browser talks to DATUM directly (upstream CORS patch)
- Sketch: upstream a DATUM patch adding configurable
  `Access-Control-Allow-Origin` and `OPTIONS` handling, then fetch `/v1/*`
  from the Svelte app with no Landfall backend involvement.
- Affected files: `landfall-web/src/lib/datum.ts`, `Datum.svelte`; upstream
  `datum_gateway/src/datum_api.c`, `datum_conf.c`.
- Tradeoffs: the smallest Landfall change, and a proxy-free web build. But it
  depends on an upstream merge and release. Browsers cannot drive Digest auth
  programmatically, so admin and config would need a new auth scheme upstream
  too. The admin password and CSRF token would sit in browser JS. Tauri's
  `tauri://` origin would also have to be allow-listed. Remote http gateways
  hit mixed-content blocks from an https-hosted web build.
- Effort: M for Landfall, plus an open-ended upstream review.

### Approach C: Native read-only stats, DATUM's own UI for admin
- Sketch: do only the public tier natively (Approach A's client without
  Digest or writes). For workers and config, deep-link to the gateway's own
  `/clients` and `/config` HTML pages. The existing `App.svelte` click handler
  already opens external links in the OS browser on desktop.
- Affected files: as in A minus the admin/config routes and the password
  storage.
- Tradeoffs: no new secret in Landfall and no write path to secure. Ships
  fast. It is not onboarding control: the miner leaves Landfall to change
  `pool_address`, and Landfall cannot detect or fix an address mismatch.
- Effort: S–M.

## Recommendation

**Approach A, shipped as three PRs.** It is the only option that meets
"Landfall is the onboarding control for DATUM" in both builds, and it fits the
existing architecture: one Rust core, httpd plus IPC transports, secrets kept
in Rust. Slice it so each PR is useful by itself:

1. **Read-only tab:** `DatumClient` with the public endpoints, settings
   persistence (URL only), the `/datum/*` read routes and Tauri commands, and
   the hidden-until-configured pill plus Settings section. This is Approach C's
   value, delivered first.
2. **Workers:** Digest SHA-256 auth, admin password in settings (`0600`),
   `/datum/clients`, and the workers table.
3. **Onboarding config:** read config, an address-mismatch banner with "Use my
   Landfall address", OCEAN presets (reward_sharing, pool host/pubkey,
   coinbase tag), and restart-aware writes. Start with these onboarding fields,
   not a generic editor for every key.

I'm fairly confident in A. The one real unknown is the Digest client (below).

## Open questions

- Digest library: does `diqwest` (or `digest_auth` driven by hand) work with
  `reqwest 0.12` and support SHA-256 `qop=auth`? It needs a spike against a
  real gateway before PR 2.
- Should PR 3 expose the bitcoind RPC fields (`rpcurl`/`rpcuser`/password),
  or stay limited to the pool and payout fields? RPC edits can brick the
  gateway remotely.
- Admin password storage on desktop: the plaintext `0600` file (matches the
  seed) or the OS keychain?
- Does the Done step get a "Mining with your own DATUM Gateway? Connect it"
  CTA? With the tab hidden, Settings alone is easy to miss.
- Is a "minimum DATUM version" check enough, or should Landfall read the
  gateway version? The JSON API has no version field today, but the HTML
  dashboard shows one since PR #173. That could become a small upstream
  addition.
