# Plan: CLI / Docker / Umbrel / Start9 deployment parity

**Goal:** Reach feature parity between the desktop app and self-hosted
deployments so a user can run the OCEAN Lightning service in any of these
shapes, with the same data showing in every UI:

1. **Desktop app** (Tauri, today's primary): already works.
2. **CLI** (`oceanln`): for scripting, headless ops, automation.
3. **Self-hosted container** (Docker / docker-compose / Umbrel / Start9):
   the same Svelte SPA served from `oceanln-httpd`, accessed in a browser
   on the user's own infra.

## Why this is feasible today

The crate split is already correct for it:

```
oceanln-common    pure Rust core — seed, sign, BIP-322, Lexe wallet ops.
                  NO transport assumption.
   ├── oceanln-cli     in-process calls
   ├── oceanln-httpd   HTTP transport (loopback today; container-ready)
   └── oceanln-web     Svelte SPA — talks to httpd (browser) OR Tauri IPC
       └── src-tauri   embeds oceanln-web, wraps common over native IPC
```

The OCEAN data the dashboard renders splits into two sources, neither of
which is desktop-specific:

| Source | Where it comes from | Transport-neutral? |
|---|---|---|
| Public stats (statsnap / earnpay / user_hashrate / pool_stat) | `api.ocean.xyz/v1/*` direct from browser, CORS-OK | Yes — works from any frontend |
| Lightning payouts | `oceanln_common::lexe_wallet::list_offer_payouts` | Yes — Rust core, **but only Tauri IPC exposes it today** |

## Concrete gaps

| # | Gap | Effort | Unlocks |
|---|---|---|---|
| 1 | HTTP route `GET /payouts` in `oceanln-httpd` + `OceanlnClient.payouts()` in the SDK | ~30 min | Web frontend (any browser) sees Lightning payouts |
| 2 | CLI command `oceanln payouts [--json]` | ~30 min | Headless automation, parity with dashboard |
| 3 | Static file serving in `oceanln-httpd` (axum's `tower-http::services::ServeDir`) | ~20 min | One container serves both API and SPA |
| 4 | Dockerfile (multi-stage Rust + npm + slim runtime) | ~1h | `docker run` works locally |
| 5 | `docker-compose.yml` reference setup | ~30 min | Single-command bring-up |
| 6 | GitHub Actions image publish to `ghcr.io` | ~1h | Pre-built images for end users |
| 7 | Umbrel community-app manifest (`umbrel-app.yml` + `docker-compose.yml`) | ~4h | Umbrel users install with one click |
| 8 | Start9 service.yaml + Makefile + s9pk packaging | ~4-6h | StartOS users install with one click |

Total: **~2 working days** across three PRs, plus review/iteration time for the
upstream Umbrel and Start9 app-store submissions (those have their own review
cadence outside our control).

---

## PR A — Server + CLI parity (≈1h)

**Branch:** `feat/payouts-route`

**Files touched:**
- `oceanln-httpd/src/lib.rs`: add `GET /payouts` route → `service::list_offer_payouts` (already exists on `WalletProvider`); auth-guard it with the existing bearer middleware.
- `oceanln-httpd/src/service.rs`: thin wrapper `list_offer_payouts(seed, wallet, limit) -> Vec<OceanPayout>` that loads the seed and delegates to `wallet.list_offer_payouts(mnemonic, limit)`.
- `oceanln-httpd/tests/server.rs`: integration test hitting `GET /payouts` (mock wallet returns a known `OceanPayout` Vec).
- `oceanln-web/src/lib/api.ts`: add `payouts(limit?): Promise<OceanPayout[]>` to the `Backend` trait + `OceanlnClient` impl (HTTP `GET /payouts`) + `TauriClient` impl (same `invoke("list_lightning_payouts")` it has today).
- `oceanln-web/src/lib/StatsGrid.svelte`: replace direct `fetchLightningPayouts()` call (Tauri-only today) with `client().payouts()` so both transports use the same `Backend` abstraction.
- `oceanln-cli/src/main.rs`: add `payouts [--limit N]` subcommand → prints OCEAN-only rows. Plain text by default, JSON with `--json`.

**Acceptance:**
- `oceanln payouts --json` prints valid JSON matching `Vec<OceanPayout>`.
- `curl -H "Authorization: Bearer $T" http://127.0.0.1:5393/payouts` returns the same shape.
- Desktop dashboard still works (regression test in `Dashboard.test.ts`).
- 67 Rust + 44 frontend tests pass + 1 new HTTP integration test for the route.

**Out of scope:** pagination, filtering by date — defer to a follow-up if a
miner ever accumulates enough payouts to need it.

---

## PR B — Containerize (≈half day)

**Branch:** `feat/docker-image`

**Files added:**
- `Dockerfile` — multi-stage build:
  - Stage 1: `rust:1.79-slim` → `cargo build --release -p oceanln-httpd`
  - Stage 2: `node:22-alpine` → `npm ci && npm run build` in `oceanln-web/`
  - Stage 3: `debian:bookworm-slim` runtime:
    - `oceanln-httpd` binary
    - `oceanln-web/dist` at `/var/www`
    - Default env: `OCEANLN_SEED_FILE=/var/lib/oceanln/seed`, `OCEANLN_STATIC_DIR=/var/www`, `OCEANLN_LISTEN=0.0.0.0:5393`
    - `VOLUME ["/var/lib/oceanln"]`
    - `EXPOSE 5393`
- `docker-compose.yml` (root) — minimal single-service reference:
  - One named volume `oceanln-data`
  - `OCEANLN_BEARER` env from `.env` (required) — token the browser uses
  - Port mapping `5393:5393`
- `.dockerignore` — `target/`, `node_modules/`, `dist/`, `.git`, worktrees
- `.github/workflows/docker.yml` — build + publish to `ghcr.io/vincenzopalazzo/oceanln-httpd:latest` on push to `main`, with semver tags from git tags
- `docs/deploy/docker.md` — operator doc: env vars, volume layout, seed init flow

**Code changes:**
- `oceanln-httpd/src/lib.rs`: add `--static-dir <path>` flag + `tower_http::services::ServeDir` middleware. Falls back to a 404 if not configured (keeps the API-only mode working).
- `oceanln-httpd`: read `OCEANLN_LISTEN` / `OCEANLN_SEED_FILE` / `OCEANLN_BEARER` / `OCEANLN_STATIC_DIR` from env as flag fallbacks (clap's `env` attribute).
- `oceanln-web`: when served from same origin as the API, default `app.base` to `''` (relative URLs) instead of `http://127.0.0.1:5393`.

**Acceptance:**
- `docker build -t oceanln .` produces a working image (< 200 MB).
- `docker compose up` starts the service; user can navigate to `http://localhost:5393`, complete the wizard (with provided bearer), and see the dashboard.
- The seed file persists across container restarts via the named volume.
- Web flow uses the new `/payouts` route from PR A and shows OCEAN-only LN payouts.
- CI publishes a tagged image on PR merge.

**Out of scope:** TLS termination (delegated to the host's reverse proxy —
Umbrel/Start9 handle this), multi-user auth (single-user container).

---

## PR C1 — Umbrel community-app submission (≈half day)

**Branch:** `feat/umbrel-app` (in `vincenzopalazzo/umbrel-apps` fork)

**Files added** (per [Umbrel community app docs](https://github.com/getumbrel/umbrel-community-app-store)):
- `umbrel-app.yml`:
  - `name: oceanln`
  - `category: bitcoin` (and `lightning`)
  - `version: 0.3.0`
  - `port: 5393`
  - Permissions: `lightning` (uses Lexe), `network` (Lexe SDK egress)
- `docker-compose.yml`:
  - Image: `ghcr.io/vincenzopalazzo/oceanln-httpd:0.3.0`
  - Volume: `${APP_DATA_DIR}/data:/var/lib/oceanln`
  - Env: `OCEANLN_BEARER=${APP_PASSWORD}` (Umbrel-provided)
  - Network: `${APP_PROXY_USERNAME}_default`
- `icon.svg` — the existing OCEAN logo
- `gallery/{1,2,3}.png` — screenshots of the dashboard, wizard, profile
- `description.md`, `release-notes.md`

**Acceptance:**
- App installs cleanly on an Umbrel test node (any of the developer kits, or a
  Pi-based home node).
- Dashboard accessible via Umbrel's auto-generated subdomain.
- Lightning payouts list populated (after the user goes through the wizard).
- Submitted to `getumbrel/umbrel-apps` community store — upstream review +
  merge is outside our control but typically 1-2 weeks.

---

## PR C2 — Start9 service packaging (≈half day to a day)

**Branch:** separate `vincenzopalazzo/oceanln-startos` repo (Start9's convention)

**Files added** (per [StartOS service packaging docs](https://docs.start9.com/latest/developer-docs/packaging-services/)):
- `manifest.yaml` — package metadata
- `Makefile` — wraps `docker build` + `s9pk pack`
- `instructions.md`, `LICENSE`, `icon.png`
- `scripts/embassy.ts` — TypeScript health checks, action handlers (mostly boilerplate)
- `assets/config_spec.yaml` — exposes the seed-import wizard / config form

**Acceptance:**
- `make` produces an installable `oceanln.s9pk`.
- Installs on a StartOS test box.
- Same dashboard works in the StartOS embedded browser pane.
- Submitted to Start9's community marketplace.

---

## Cross-cutting work

These touch every PR; track separately so they don't get lost:

- **Bearer-token UX in browser mode.** Desktop has no token; browser/server
  mode needs the user to enter one. PR A already triggered the fix
  ([App.svelte $effect on app.token](oceanln-web/src/App.svelte) in commit
  c0f6a12) — verify it still works in the containerized flow.
- **Seed-init flow in container.** The wizard's `generate` / `import` writes
  to the configured seed path. The container needs that path to be a mounted
  volume. Document: "first launch, complete the wizard, your seed will be
  written to the mounted volume — back it up like you would any other LN
  wallet seed." Default seed-file permissions are 0600 — already enforced.
- **No-Tauri build cleanup.** The Svelte frontend currently has both a
  `TauriClient` and an `OceanlnClient`. Both stay — the runtime
  `isTauri()` check picks one. No #ifdef needed; bundle size cost is
  negligible (~2KB after gzip).

## Risk register

- **Image size.** The Lexe SDK pulls a heavyweight dep tree (lexe-public's
  workspace is ~30+ crates including SGX-related stubs). A naive `cargo build
  --release` binary might be 40-80 MB. Mitigation: strip + LTO (already in
  `profile.release`); accept the size — for self-hosted Lightning, it's not
  unusual.
- **Lexe SDK requires outbound HTTPS to `gateway.lexe.app` / `run.lexe.app`.**
  Confirm Umbrel and Start9 firewall rules allow this; Tor-only nodes need
  Lexe to support Tor exits (they don't yet, per docs check needed).
- **Seed file backup.** Containers eat data when volumes are removed. Make the
  README scream "back up the seed phrase the wizard shows you — the container
  volume is NOT a backup."

## Order of operations

1. **Now**: PR A on this repo. Small, reviewable, unlocks browser-mode
   feature parity with desktop.
2. **After A merges**: PR B in the same repo. Image + compose. Test locally
   with `docker compose up`.
3. **After B publishes images**: PR C1 (Umbrel) and PR C2 (Start9) in
   parallel — different upstream repos, no merge-order coupling.
4. **Optional capstone**: a CI job that runs `oceanln payouts --json` against
   a mock httpd to lock in CLI/HTTP contract parity. Saves us from drift.

## Estimated total

- Engineering: **2 working days** end-to-end for PRs A + B + C1 + C2.
- Calendar: **2-4 weeks** if you want upstream merges in the Umbrel and
  Start9 community stores (their review cadence dominates).
