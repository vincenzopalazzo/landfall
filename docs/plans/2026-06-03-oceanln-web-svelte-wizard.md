# Plan: oceanln-web Svelte onboarding wizard + httpd /generate · /import

Source brainstorm: `docs/brainstorms/2026-06-03-oceanln-web-svelte-wizard.md`
Design reference (in-repo): `docs/design/ocean-lightning-wizard/`

**Goal:** Ship the OCEAN Lightning onboarding design as a real Svelte + Vite (TS) SPA in
`oceanln-web/` driving `oceanln-httpd`, adding loopback-only `POST /generate` and
`POST /import` so the wizard can create/import a wallet; dashboard + MCP panel are mocked;
the app stays Tauri-ready.

> PR #7 is merged into `main`; this worktree branch is 1 commit behind. **Start
> implementation from a fresh branch off `origin/main`** (or rebase this worktree first).

## Recommended PR split

Build as **three independent PRs** — do not land it all at once:

- **PR A (backend, Rust):** `POST /generate` + `POST /import` in `oceanln-httpd` + tests. **S–M.**
- **PR B (frontend skeleton):** `oceanln-web/` scaffold + design-system port + the wizard
  wired to real endpoints (`/generate`,`/import`,`/init`,`/offer`,`/payout`). **L.**
- **PR C (surfaces):** Profile + Lightning dashboard + MCP panel (mock data), Tweaks. **M.**
- (Later, separate) Tauri shell. Out of scope here.

This plan covers PR A and PR B in detail (the critical path); PR C is sketched.

---

## PR A — backend: `/generate` + `/import`

**Affected files:**
- `oceanln-httpd/src/lib.rs` — two new handlers + routes; `GenerateResp`/`ImportReq`/`ImportResp`.
- `oceanln-common/src/sign.rs` — no change expected; reuses `generate_mnemonic`,
  `store_seed(secret, dest, force)`, `parse_mnemonic`, `parse_bip32_path`, `derive_address`.
- `oceanln-httpd/tests/server.rs` — bound-socket tests for both endpoints.
- `README.md` — document the two endpoints + the wizard flow.

**Endpoint contracts** (both behind the existing guard: loopback bind + bearer token +
Origin allowlist + Host check):

- `POST /generate`  body `{ force?: bool }`  →  `{ mnemonic: String, mining_address: String }`
  - Generate via `sign::generate_mnemonic()`, parse, `sign::derive_address(&m, &default_path)`.
  - Persist with `sign::store_seed(&secret, None, force)`. `store_seed` already **refuses to
    clobber a different existing seed unless `force`** (and is idempotent for an identical
    one) — surfaces as `Error::Wallet` → HTTP 500 today; map seed-conflict to **409**
    (see error-mapping change below).
  - Reveal the phrase **once** in the response; log to stderr that a phrase was revealed
    (no token/seed contents in the log).
- `POST /import`  body `{ mnemonic: String, force?: bool }`  →  `{ mining_address: String }`
  - `MnemonicSecret::from_input` is gone (removed earlier); build the secret via the seed
    path — simplest: write through `store_seed`. Validate 24 words with `parse_mnemonic`
    first (400 on invalid), then `store_seed(&secret, None, force)`, then derive + return
    the address. Seed crosses the wire once, by design.

**Error mapping change** (`ApiError::into_response` in `lib.rs`): add a way to return **409
Conflict** for a "seed already exists" case so the wizard can distinguish "already set up"
from a real failure. Options: a new `Error::SeedExists`-style variant in
`oceanln-common::error`, or detect in the handler and return a tagged `ApiError`. Prefer a
small dedicated path in the handler (avoid widening the shared `Error` enum unless reused).

**`store_seed` is `#[cfg(feature = "lexe-sdk")]`** — fine, `oceanln-httpd` always enables it.
Keep `/generate`/`/import` un-gated in the router (the lib already assumes the feature).

**Wallet seam:** generate/import are `sign`-only (no `WalletProvider`), so they need no mock;
existing `MockWallet` in tests still covers `/offer` + `/init`.

**Tests (`oceanln-httpd/tests/server.rs`, existing bound-socket harness):**
- `/generate` with no existing seed → 200, body has 24-word `mnemonic` + `bc1q…` address;
  the seed file now exists.
- `/generate` again (different/no force) → **409** (no-clobber), and the response does NOT
  contain a second mnemonic.
- `/generate` idempotent: identical-seed case is a no-op (covered by `store_seed` semantics).
- `/import` valid 24 words → 200, deterministic `bc1qpstw48j7j9gjugw25jmjvd96jlwgdnedk5pr6r`.
- `/import` invalid (12 words / garbage) → 400.
- Both require the bearer token (401 without) and respect the Origin allowlist (403).
- Reuse the temp-seed 0600 fixture pattern already in the test file; each test uses a unique
  seed-file path and passes it via `ServerConfig.seed` so tests don't collide.

**Edge cases / risks:**
- **Seed reveal over the wire** — gated by loopback + token + Origin; `/generate` no-clobber;
  stderr "phrase revealed" log. Documented as an accepted relaxation of the prior model.
- A test that calls `/generate` writes a real seed file — point `ServerConfig.seed` at a
  temp path (the handler must persist to *that* path, not the default). **Implementation
  note:** `store_seed(secret, None, force)` writes to the *default* config path, ignoring
  the configured `SeedSource`. For `/generate` to honor the server's `--seed-file`, the
  handler must pass the configured path as `dest`. So `ServerConfig` needs the seed-file
  **path** (not just a `SeedSource`), or `SeedSource::File(path)` must expose its path.
  → Add `SeedSource::path() -> Option<&Path>` (or store the `PathBuf` in `ServerConfig`) and
  pass it to `store_seed`. This is the one non-obvious wiring detail.
- Concurrency: two simultaneous `/generate` calls — `store_seed` no-clobber makes the second
  fail cleanly; acceptable for single-operator.

**Estimated size:** S–M (~120–180 LOC incl. tests).

---

## PR B — `oceanln-web/` Svelte SPA (wizard, real endpoints)

**Scaffold:** `npm create vite@latest oceanln-web -- --template svelte-ts` (Node v25/npm 11
present). Add `oceanln-web/.gitignore` (`node_modules`, `dist`, `.env.local`). **Keep it out
of the Cargo workspace** (do not add to root `Cargo.toml` members) and **out of the Rust CI
job**.

**Affected files (new):**
- `oceanln-web/package.json`, `vite.config.ts`, `tsconfig.json`, `index.html`, `.gitignore`.
- `oceanln-web/src/app.css` ← port `docs/design/ocean-lightning-wizard/project/colors_and_type.css`
  + the `<style>` block from `Ocean Lightning Wizard.html` (window chrome `.wz-*`, rail,
  content, callouts, pills, copy fields). Keep accent as CSS custom props for the Tweaks toggle.
- `oceanln-web/public/fonts/*` ← `project/fonts/*`; `oceanln-web/public/assets/*` ← `project/assets/*.svg`.
- `oceanln-web/src/lib/api.ts` — typed client: `health()`, `generate(force?)`, `import(mnemonic,force?)`,
  `init(path?)`, `offer(description?,minAmount?)`, `payout({message,offer?,…})`. Reads base
  URL + bearer token from `config.ts`; throws a typed error carrying the `{error}` body + status.
- `oceanln-web/src/lib/config.ts` — base URL (default `http://127.0.0.1:7762`) + token from
  `import.meta.env.VITE_OCEANLN_TOKEN` **or** a paste-field value (see open question).
- `oceanln-web/src/lib/state.ts` — Svelte stores: wizard step, gating/validation, collected
  artifacts (address/offer/signature), accent/explainer Tweaks.
- `oceanln-web/src/lib/ui/` — `WindowChrome.svelte`, `StepRail.svelte`, `Tooltip.svelte`
  ("what's this?"), `CopyField.svelte`, `Callout.svelte`, `Icon.svelte` (port `wizard/ui.jsx`).
- `oceanln-web/src/lib/wizard/` — `Welcome`, `RecoveryPhrase` (create via `/generate` or
  import via `/import`; reveal + back-up warnings), `ConfirmBackup` (reselect words;
  functional store updates so rapid clicks don't clobber), `CreateWallet` (description chips
  → `/offer`; overlay the `PROVISION_TASKS` progress while the real call runs; `/init` for
  the address), `SignForOcean` (`/payout` with the embedded `offer`), `Done` (3 `CopyField`s
  + "Verify now") (port `wizard/steps.jsx`).
- `oceanln-web/src/App.svelte` — window chrome + step rail + content switch (port `wizard/app.jsx`).
- `oceanln-web/src/main.ts` — mount.
- `oceanln-web/README.md` — run instructions.

**Approach:**
- Recreate the prototype's **visual output** pixel-fairly; do not copy its Babel/React
  internals. Svelte components mirror the `.jsx` modules 1:1 for reviewability.
- Real data flow: `RecoveryPhrase` → `/generate` (or `/import`); `CreateWallet` → `/offer`
  (description embedded) + `/init` (address); `SignForOcean` → `/payout` (offline `offer`
  mode, message embeds the offer). `Done` shows the three **real** artifacts with copy + verify.
- The simulated `PROVISION_TASKS` progress animation runs *concurrently with* the real
  `/offer`/`/init` calls (cosmetic; gate "Continue" on the real promise resolving).
- Keep state-correct visuals off pure CSS-transition timing (lesson from the design chat);
  Svelte's reactivity avoids the React capture quirks.

**bitcoin.design alignment (from the chat):** plain "recovery phrase" language, education
before backup, screenshot/share warnings, confirm-by-reselecting words, "what's this?"
tooltips, big hit targets.

**Run / serving (separate origin):**
```sh
# terminal 1 — backend, allow the Vite dev origin
oceanln-httpd --seed-file ./seed.txt --allow-origin http://localhost:5173
# terminal 2 — frontend
cd oceanln-web && npm install && npm run dev
```
Token reaches the page via `VITE_OCEANLN_TOKEN` (dev) or a paste field (open question).

**Test plan:**
- `npm run build` (tsc + vite) must pass — this is PR B's CI gate.
- `npm run check` (svelte-check) clean.
- Manual end-to-end against a running `oceanln-httpd`: generate → backup → confirm → offer →
  sign → copy 3 artifacts; import path; 409-already-setup handling; bad-token surfaced as a
  clear UI error.
- (Optional) a Vitest unit test for `api.ts` error mapping and `state.ts` confirm-step logic.

**Edge cases / risks:**
- **CORS/token in dev**: the Vite origin must be in `--allow-origin`; bad/missing token must
  surface a readable message, not a silent failure.
- **JS toolchain in a Rust repo**: isolate under `oceanln-web/`; do not touch the Rust CI
  job. Add a separate web CI lane (build + svelte-check) in a follow-up if desired.
- **Tauri-ready**: no SSR; only browser/`fetch` APIs; relative asset paths; so a later
  `src-tauri/` can bundle `dist/` and add the Tauri origin to the allowlist unchanged.
- Design assets are fonts/SVGs (binary) — copy as-is from `docs/design/.../project/`.

**Estimated size:** L (>200 LOC across many small components).

---

## PR C — Profile + Dashboard + MCP (mock) — sketch

- `oceanln-web/src/lib/data.ts` ← port `wizard/data.jsx` `DASH`/`MCP` mocks, **clearly
  labelled illustrative**.
- `lib/profile/Profile.svelte` (port `wizard/profile.jsx`): 1→n onchain addresses each with a
  "Linked offer" selector; offers list; reveal-phrase (re-`/generate`? no — reveal must come
  from a stored-seed read; **mock or a future `/reveal` endpoint** — flag); tiles to dashboard
  + re-run setup. Drop "Back to setup" per the final chat turn.
- `lib/dashboard/Dashboard.svelte` (port `wizard/dashboard.jsx`): node status, stats, payouts
  table, offer card, MCP panel (`claude mcp add oceanln …` — illustrative; `oceanln mcp serve`
  does not exist).
- `lib/Tweaks.svelte` (port `tweaks-panel.jsx`): explainer density, 24/12-word, accent.
- Top-nav surface switch in `App.svelte` (wizard ↔ profile ↔ dashboard).

**Open question for PR C:** "reveal recovery phrase" on the Profile needs the stored seed —
either a new loopback-only `GET /reveal` (another seed-over-wire surface) or keep it mocked.
Decide before building Profile.

---

## Conventions to follow

- Rust: errors via the shared `Result`/`Error`; no `unwrap()` in prod paths; constant-time
  token compare already in place; keep `#[cfg(feature="lexe-sdk")]` discipline; bound-socket
  integration tests over `tower::oneshot`.
- No `Debug` on secret-bearing structs.
- Repo naming: `oceanln-*` kebab. Frontend dir `oceanln-web/`.
- CI: Rust job unchanged; do not add JS to it. README documents the web run/build.

## Open questions / risks (carried)

1. **Seed over the wire to a separate origin** — accepted; mitigated by loopback + token +
   Origin allowlist + `/generate` no-clobber + reveal log. Re-confirm before PR A merges.
2. **Dev token delivery** — `VITE_OCEANLN_TOKEN` vs paste field. (Recommend: support both —
   env for convenience, paste field as fallback.)
3. **Profile "reveal phrase"** — mock vs a new `/reveal` endpoint (more seed exposure). Defer
   to PR C decision.
4. **MCP panel** — illustrative mock vs cut. (Recommend: keep, clearly labelled.)
5. **`/generate` must write to the configured `--seed-file`**, not the default path — requires
   exposing the seed path to the handler (the one real wiring gotcha in PR A).

## Estimated total size

L overall. Critical path PR A (S–M) + PR B (L); PR C (M) and Tauri (separate) follow.
