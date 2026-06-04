# Brainstorm + Plan: oceanln-web Svelte onboarding wizard

Date: 2026-06-03

Implements the Claude Design handoff **"Ocean Lightning onboarding flow (aka Wizard)"**,
preserved in-repo at `docs/design/ocean-lightning-wizard/` (read its `README.md`,
`chats/chat1.md`, `project/Ocean Lightning Wizard.html`, and `project/wizard/*.jsx`).

## Clarified Problem Statement

**Goal:** A Svelte web app implementing the OCEAN Lightning onboarding design (wizard +
profile + payout dashboard + MCP panel) as a real frontend driving `oceanln-httpd`,
structured so it can later be wrapped in a Tauri desktop app.

**Decisions (locked):**
- **Stack:** Svelte + Vite (TypeScript) **SPA**, no SSR. New top-level crate-sibling dir
  `oceanln-web/`. Build to static assets so a later Tauri shell bundles them unchanged.
- **Seed:** add a **loopback-only `POST /generate`** to `oceanln-httpd` that creates the
  24-word phrase, persists it to the seed file, and returns it **once** so the wizard can
  reveal + back it up. Add **`POST /import`** for the "I already have a phrase" path. This
  intentionally relaxes the prior "seed never crosses the wire / generation is CLI-only"
  rule — accepted, gated by loopback + bearer token + Origin allowlist.
- **Serving:** UI is **hosted separately** (its own origin). It calls `oceanln-httpd`
  cross-origin, so the server's `--allow-origin` must include the Vite dev origin (and
  later the Tauri origin). Bearer token reaches the page out-of-band (see open questions).
- **Scope:** **full design, mock where the backend can't power it** — wizard is real;
  dashboard payout data + MCP (`oceanln mcp serve`) panel are illustrative mocks.

**Constraints:**
- Port the design's visual system verbatim from `docs/design/ocean-lightning-wizard/project/`:
  `colors_and_type.css`, fonts (`fonts/`), OCEAN SVGs (`assets/`), `#09090b` surface,
  bitcoin-orange accent (`--wiz-accent:#f7931a`), window-chrome + left step-rail layout.
  Recreate pixel-fairly; do **not** copy the prototype's Babel/in-browser-React structure.
- bitcoin.design-aligned: plain-language "recovery phrase" (not seed/mnemonic), education
  before backup, screenshot/sharing warnings, confirm-by-reselecting words, "what's this?"
  tooltips, large hit targets. Audience = less-technical miners.
- The 3 produced artifacts (BIP84 address, `lno1…` offer, BIP-322 signature) must be real,
  from `oceanln-httpd`, with copy buttons + a "Verify now" hand-off.

**Non-goals:**
- No real payout monitoring and no MCP server (backend has neither) — mock, clearly labelled.
- Not shipping the Tauri shell this pass; only keeping the app Tauri-ready.
- No change to the CLI seed flow.

**Success criteria:**
- `oceanln-httpd --seed-file … --allow-origin <vite-origin>` running, the Svelte app
  completes end-to-end: generate (or import) → back up → confirm → create wallet
  (description→offer) → BIP-322 sign OCEAN message → copy 3 real artifacts + Verify.
- Profile (1→n addresses, offers, reveal phrase, links) and Lightning dashboard render per
  design; live where the backend allows, mock otherwise.
- `npm run build` emits static assets; runs in a browser today; no Tauri-only API blocks a
  later wrap.

## Backend changes (`oceanln-httpd` + `oceanln-common`)

Reuse existing `/health`, `/init`, `/offer`, `/payout` (and their guard: loopback bind,
constant-time bearer token, Origin allowlist, Host check). Add:

1. **`POST /generate`** → `{ mnemonic: String, mining_address: String }`
   - Generates a fresh 24-word phrase (`sign::generate_mnemonic`), derives the BIP84
     address, persists via `sign::store_seed` (0600), returns the phrase once.
   - **Refuses (409/400) if a seed file already exists** unless an explicit `force` flag is
     set — mirror `store_seed`'s no-clobber rule. Log to stderr that a phrase was revealed.
   - `WalletProvider` is unaffected (generation is `sign`-only); reveal happens before any
     `provision`.
2. **`POST /import`** `{ mnemonic: String }` → `{ mining_address: String }`
   - Validates 24 words, persists to the seed file (no-clobber unless `force`). Returns the
     derived address. (The seed crosses the wire once, by design.)
3. CORS/guard: confirm `OPTIONS` preflight already works (covered by
   `oceanln-httpd/tests/server.rs`); add `/generate` + `/import` to the integration tests
   via the existing bound-socket harness + `MockWallet` (generate/import don't need the
   wallet). Assert no-clobber and that `/generate` reveals the phrase exactly once.

Endpoint ↔ wizard-step map:

| Wizard step | Endpoint |
|---|---|
| Recovery phrase (create) | `POST /generate` |
| Recovery phrase (import) | `POST /import` |
| Create wallet (description→offer + provision) | `POST /offer` (+ `POST /init`) |
| Derive payout address | from `/init` / `/generate` response |
| Sign for OCEAN | `POST /payout` (offline `--offer` mode) |
| Dashboard / MCP | **mock** (no backend) |

## Frontend `oceanln-web/` (Svelte + Vite + TS)

Scaffold: `npm create vite@latest oceanln-web -- --template svelte-ts`. Add `.gitignore`
for `node_modules`/`dist`. Not a Cargo workspace member.

**Design-system port** (from `docs/design/ocean-lightning-wizard/project/`):
- `src/app.css` ← `colors_and_type.css` + the `<style>` block in `Ocean Lightning Wizard.html`
  (window chrome `.wz-*`, rail, content, callouts, pills, copy fields, dashboard, profile).
- `static/fonts/` ← `fonts/*` (Geist Mono, Inter), `static/assets/` ← OCEAN SVGs.
- CSS custom props for the accent so the Tweaks toggle (orange ↔ ocean-blue) works.

**Components** (recreate the `.jsx` modules as `.svelte`):
- `lib/api.ts` — typed client for `oceanln-httpd` (`generate`, `import`, `init`, `offer`,
  `payout`, `health`); base URL + bearer token from config; surfaces `{error}` bodies.
- `lib/config.ts` — base URL + token resolution (env / paste field).
- `lib/ui/` — `Tooltip.svelte` ("what's this?"), `CopyField.svelte`, `Callout.svelte`,
  `Icon.svelte`, `WindowChrome.svelte` (titlebar + lights), `StepRail.svelte` (port `ui.jsx`).
- `lib/wizard/` — `Welcome`, `RecoveryPhrase` (reveal + create/import), `ConfirmBackup`
  (reselect words, functional state per the chat's hardening), `CreateWallet`
  (description chips → offer → simulated `PROVISION_TASKS` progress while real calls run),
  `SignForOcean`, `Done` (3 copy fields + Verify) (port `steps.jsx`).
- `lib/profile/Profile.svelte` — 1→n onchain addresses each linkable to an offer, offers
  list, reveal-phrase, tiles to Lightning dashboard + re-run setup (port `profile.jsx`).
- `lib/dashboard/Dashboard.svelte` — node status, stats, payouts table, offer card, MCP
  panel (**mock data** from a ported `data.ts`; MCP card labelled illustrative) (port
  `dashboard.jsx`).
- `lib/Tweaks.svelte` — explainer density, 24/12-word, accent (port `tweaks-panel.jsx`).
- `App.svelte` — window chrome + surface switch (wizard ↔ profile ↔ dashboard) + top nav
  (port `app.jsx`; drop "Back to setup" per the final chat turn).
- `lib/state.ts` — wizard step state, gating/validation, derived artifacts (Svelte stores).

**Mock vs real matrix:**
- Real: generate/import, offer (with description), payout (sign), derived address, health.
- Mock: `DASH` payout monitoring, `MCP` connect panel — ported as static `data.ts`, visibly
  marked illustrative.

## Build order (each independently shippable)

1. Backend: `POST /generate` + `POST /import` in `oceanln-httpd` lib + `WalletProvider`-free
   `sign` calls; integration tests (no-clobber, single reveal). **S–M.**
2. `oceanln-web/` scaffold + design-system port (fonts, CSS, window chrome, step rail) — a
   static Welcome screen that matches the prototype. **M.**
3. Wizard wired to real endpoints (generate/import → offer → sign → hand-off), with the
   simulated provisioning progress overlaying the real calls. **M–L.**
4. Profile + Lightning dashboard + MCP (mock data, real where available). **M.**
5. README: how to run (`oceanln-httpd --allow-origin http://localhost:5173` + `npm run dev`),
   token delivery, build. **S.**
6. (Follow-up, separate PR) Tauri shell: `src-tauri/`, bundle the static build, run
   `oceanln-httpd` in-process, add the Tauri origin to the allowlist. **L.**

## Risks / open questions

- **Seed over the wire to a separate origin** widens exposure vs the old CLI-only model.
  Mitigations: loopback bind, bearer token, Origin allowlist, `/generate` no-clobber,
  stderr log on reveal. Confirm this posture is acceptable.
- **Dev token delivery:** paste-the-token field in the UI, or `VITE_OCEANLN_TOKEN` env for
  local dev? (Tauri later removes the question — same process.)
- **MCP panel:** keep as a labelled "coming soon" mock, or cut from this pass? (`oceanln mcp
  serve` does not exist.)
- **JS toolchain in a Rust repo:** `oceanln-web/` adds Node/npm. Keep it out of the Cargo
  workspace and out of the Rust CI job; optionally add a separate web CI lane later.
- **Confirm-step + provisioning robustness:** the design chat hit React capture/animation
  quirks (entrance opacity, rail transitions, batched clicks). In Svelte these mostly vanish,
  but keep state-correct visuals off pure CSS-transition timing and use functional store
  updates for rapid word selection.
