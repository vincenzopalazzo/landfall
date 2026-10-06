# Pre-release review — oceanln as an open-source BIP-322 wallet example

Date: 2026-10-06. Branch reviewed: `claude/cool-ride-s0pu2k` at `063df51`.
Scope: every crate, the web wizard, the docs site, the Tauri shell, the mobile
tree, scripts, CI and repo hygiene. Read-only review; no code was changed.

Baseline (all run in this review): `cargo build --workspace`, `cargo fmt
--check`, `cargo clippy --all-targets --all-features -D warnings`, `cargo test
--all-targets --all-features` (107 tests), `npm test` in `oceanln-web` (60
tests), `npm run build`, `npm audit` (0 vulns). Everything is green.

Severity key: **Blocker** = fix before the public tag. **High** = fix in the
first release cycle. **Medium/Low** = quality and polish.

> **Status (same day, later commits on this branch):** all seven blockers
> below are fixed (B1 `bip322` 0.0.12 + key/address binding + `verify`;
> B2 `--no-auth` gating; B3 probe default; B4 skill file; B5 README, docs
> site and MSRV; B6 pagination; B7 mobile banner). A QA harness now pins
> them: `docs/QA-SCENARIOS.md` is the scenario registry, `scripts/qa/`
> drives the CLI, the HTTP server and the web wizard (headless Chromium,
> then `oceanln verify` on the signature the wizard showed), and
> `.github/workflows/qa.yml` runs it in CI. The UX items in section 2 and
> the medium/low items are still open and are tracked as "open" scenarios
> in the registry (QA-011, QA-210…213).

---

## 1. Release blockers

### B1. The pinned `bip322` 0.0.10 verifier does not bind the key to the address
`oceanln-common/src/sign.rs:410-417` hands any P2WPKH address to
`sign_simple_encoded`. The crate's `verify_full_p2wpkh` reads the pubkey from
`witness[1]` and compares it with itself (`bip322-0.0.10/src/verify.rs:131-135`);
it never checks `wpubkey_hash()` against the address's script. Consequences:

- `sign_bip322(key, wrong_address, msg)` returns a signature, and the crate's
  own `verify_simple_encoded` says `Ok`. A correct verifier (OCEAN's) rejects it.
- The round-trip tests (`tests/bip322_vectors.rs:35-43`, `sign.rs:515-517`)
  therefore cannot detect a wrong-key or wrong-path regression.

Fix: bump to `bip322 = "0.0.12"` (workspace `Cargo.toml:24`), which compares
`ScriptBuf::new_p2wpkh(&pub_key.wpubkey_hash())` against the prevout. In
`sign_bip322` either assert `address_from_key(key)? == address` or drop the
`address` parameter and derive it. Add a negative test (wrong key must fail).

Sign-side output is spec-correct: the Bitcoin Core vector (`L3VF…`,
"Hello World") reproduces `AkgwRQIhAOzy…` byte-for-byte, the BIP-84 vector
mnemonic derives `bc1qcr8te4kr609gcawutmrza0j4xv80jy8z306fyu`, and the
`BIP0322-signed-message` tagged hashes match the BIP. Pin these as
known-answer tests (`sign.rs:505-518` only checks the `bc1q` prefix today).

### B2. `--no-auth` leaves wallet-replacing and key-signing routes open
`oceanln-httpd/src/lib.rs:494-504`: with an empty token the shared `guard`
skips the bearer check. Only `/pay` and `/seed/reveal` get the extra
`require_token` layer (`lib.rs:550-559`). So in `--no-auth` mode any local
process with no `Origin` header can call `POST /import {"force":true}` (swap the
seed), `POST /payout` (obtain BIP-322 signatures from the mining key) and
`POST /generate`. Fix: attach `require_token` to `/generate`, `/import`,
`/payout`, `/offer`, `/init`, `/invoice` too, or define `--no-auth` as
read-only (`/status`, `/payouts`, `/node`, `/activity`, `/ocean/*`).

### B3. A real miner payout address is the default in a script
`scripts/ocean-probe.sh:8` defaults to `bc1qarcmp3a5adjru4v8rh2f6mfqu6qkxepusxnqlg`,
a live OCEAN address that appears nowhere else in the repo. Fix:
`ADDR="${1:?usage: ocean-probe.sh bc1q...}"`.

### B4. Personal paths and stale claims in the project skill
`.claude/skills/oceanln.md:14,16,21,27` hardcode
`/Users/vincenzopalazzo/github/work/btc/oceanln-cli`. The same file says the
CLI has "exactly two commands" (there are five: `generate`, `payout`, `init`,
`offer`, `payouts`), calls `--url`/`--credentials` global flags (they are
`payout`-only), says the default build is the thin sidecar client
(`oceanln-cli/Cargo.toml` defaults to `lexe-sdk`), and recommends
`cargo install --path .` which the README itself says fails. Rewrite with
relative paths from `git rev-parse --show-toplevel`.

### B5. README contradicts the code in ways a first reader will hit
- `README.md:190` "seed generation … deliberately not exposed over HTTP" vs
  `README.md:216` and `lib.rs:535` (`POST /generate`). Delete the sentence.
- `README.md:258` and `oceanln-docs/.../docs/wizard/+page.svelte:30` say MCP is
  "`oceanln mcp serve`, not yet built"; `oceanln-mcp` exists as a
  Streamable-HTTP proxy on `127.0.0.1:7763/mcp` with seven read-only tools.
- `README.md:9` "three crates"; the workspace has four. `README.md:34` omits
  the `oceanln-mcp` binary.
- The endpoint table (`README.md:212-220`) and the docs-site `http-api` page
  omit `/status`, `/payouts`, `/node`, `/activity`, `/invoice`, `/pay`,
  `/ocean/*`. Regenerate from the router in `lib.rs:534-566`.
- The CLI `payouts` subcommand is undocumented everywhere.
- `rust-version = "1.74"` (`Cargo.toml:15`) is false: the Lexe 0.1.18 crates
  declare `rust-version = "1.90"`. Set 1.90 or drop the field.

### B6. Lexe payout listing truncates from the wrong end
`oceanln-common/src/lexe_wallet.rs:183-227` and `:381-414` stop paginating as
soon as `out.len() >= limit` while `get_updated_payments` returns ascending
order, then sort newest-first. A user with more than `limit` payouts sees
their oldest payouts presented as the most recent. Fix: walk to the tail
(bounded by `MAX_PAYMENTS_SCANNED`), sort descending, then `truncate(limit)`.

### B7. Mobile must be labelled experimental in the README it ships with
The commit message for `063df51` says "EXPERIMENTAL, not production-hardened"
and that Send "has never been exercised against a funded wallet", but neither
`mobile/README.md` nor the root README says so. The seed is a plain `0600`
file (`mobile/core/src/lib.rs:383`), no Keychain or Keystore, the phrase
screen has no `FLAG_SECURE` and no iOS app-switcher blur, and `mobile.yml`
never builds the `.so` or the Xcode project. Add a prominent banner, or move
the tree to a branch.

---

## 2. The BIP-322 UX: what to change so the example teaches the right thing

The flow has the right bones: one seed, reveal then confirm, auto-provision,
sign a pasted message, hand off. These are the places where it stops being a
good example.

### U1. The seed is persisted before the user has backed it up, and three paths dead-end
- `oceanln-web/src/lib/wizard/Phrase.svelte:11-15` calls `/generate` on mount,
  which writes the seed server-side.
- Close the tab before backing up: `store.svelte.ts:255-260` routes a
  "configured, no offer" server to Import and demands 24 words the user never
  saw. `/seed/reveal` and `loadPhrase()` exist but are not offered here.
- Create, Back, "I already have a phrase": `importWallet`
  (`store.svelte.ts:363-368`) never sends `force`, the server 409s, and the UI
  has no force path.
- Typing in Settings: `App.svelte:68-72` re-runs `bootstrap()` on every
  keystroke of the `bind:value` token/base inputs (`Settings.svelte:16-17`);
  each retarget calls `resetWalletIdentity(true)` which clears `app.phrase`
  (`store.svelte.ts:304`) and rewinds to Welcome.

Recommendation: on any "seed exists, no offer" state show "Reveal and back up
the stored phrase"; on import-409 show a confirmed "Replace the wallet on this
server?" and retry with `force: true`; commit Settings on blur or Enter.

### U2. The signing step never shows what was signed
`app.message` (the exact bytes echoed by `/payout`) is stored at
`store.svelte.ts:419` and rendered nowhere. After signing, the textarea is
disabled and `goBack` never clears `signature` (`Sign.svelte:52,64`,
`store.svelte.ts:126-131`), so a wrong paste cannot be redone without "Re-run
setup". This is the pedagogical heart of a BIP-322 example and it is silent.

Recommendation: merge Sign and Done into one "Prove ownership" screen:
address and offer at the top, paste box, then a block reading "You signed
exactly this text with bc1q…" followed by the verbatim message and one
signature copy field, plus "Sign a different message".

### U3. The backup quiz proves nothing
`Confirm.svelte:29-33,52-56` is a four-option multiple choice with instant
right/wrong colouring; each word is brute-forceable in three clicks. Use
free-text entry for the three positions, validated only on Continue.

### U4. Repetition and fake controls
Address and offer are copy fields on three consecutive screens
(`Sign.svelte:23-34`, `Done.svelte:41-43`, `Wallet.svelte:101-114`).
`Profile.svelte:30-53,105-110` has "Derive address" (inserts a placeholder
row), "New offer" (creates a real server offer that vanishes on reload) and
"Linked offer" (local-only state). An example should only demonstrate real,
persisted behaviour; strip these.

### U5. Jargon in primary labels
`data.ts:22-24`, `Sign.svelte:24,34,77`, `Wallet.svelte:97-98,101,114`,
`StatsGrid.svelte:438-439` show "BOLT12", "lno1", "BIP-322", "bc1q", "TIDES",
"sealed enclave", "Lexe" regardless of the Guided/Concise toggle. Use
"Lightning address", "Payout address", "Proof of ownership" as labels and keep
the standard names in tooltips.

### U6. Naming drift across transports
`oceanln-cli/src/cli.rs:47,65`: `payout` (sign an authorization) and
`payouts` (list received payouts) differ by one letter and point in opposite
directions; HTTP and Tauri repeat the pair. CLI `init` means
generate-or-read + store + provision, while HTTP `/init` is provision-only and
HTTP `/import` is what CLI `init` with stdin does. Consider `sign` or
`authorize` for the signing flow (keep `payout` as a hidden alias for one
release) and add CLI `import`/`status`/`reveal` to mirror the HTTP routes.

### U7. Smaller web items
- `Dashboard.svelte:28,126-128` renders the bearer token in plaintext into the
  MCP command and copies it to the clipboard.
- `Profile.svelte:16-28`: "Hide" flips `revealed` but leaves the phrase in
  `app.phrase` for the session.
- `StatsGrid.svelte:167-172` swallows `payouts()` failures so "Total paid"
  silently under-reports.
- `App.svelte:95-101` rail steps are `role="button"` with `tabindex="-1"`.
- No tests cover `Sign`, `Done`, `Confirm`, `Settings`, import paste, or
  goBack-after-sign.

---

## 3. Core crate (`oceanln-common`)

Solid: 0600 and `O_EXCL` seed files with 0700 parent, `MnemonicSecret` without
`Debug`/`Clone` and zeroized on drop, bip39 errors report only the word index,
timeouts on all three HTTP clients, rustls with bundled roots, no secret ever
logged (grep clean), loopback checks handle `127.evil.com`.

- **High** B1 and B6 above.
- **Medium** `service.rs:161-173`: the create-offer branch of `payout` uses
  the sidecar client while everything else goes through `WalletProvider`, and
  it signs the caller's `message` unchanged, so the returned `{offer,
  signature}` pair is an offer the signature does not authorize. The
  offer-in-message check at `:152-158` only runs for the caller-supplied
  branch. Route through `WalletProvider::create_offer` or drop the branch.
- **Medium** `service.rs:67-72,85-89`: `GenerateResp` and `RevealResp` derive
  `Debug` and hold the mnemonic as a plain `String`. Redact in a manual
  `Debug` and use `Zeroizing<String>`.
- **Medium** `sign.rs:123-160`, `ocean.rs:200-214`: seed-file, OCEAN API and
  config-dir failures are all wrapped in `Error::Wallet` ("lexe wallet: …")
  and mapped to 500. Add `SeedFile` and `Ocean` variants.
- **Medium** `sign.rs:155,221`, `service.rs:390,422`: `store_seed`,
  `generate`, `import` are gated on `lexe-sdk` although they never touch
  Lexe; the README's thin-build promise is broken.
- **Low** `sign.rs:325-330`: `master` and `child` `Xpriv` are not erased;
  `service.rs:182` relies on `non_secure_erase`, which is best-effort. Say so
  in the doc comment.
- **Low** `sign.rs:67` reads one line from stdin while the file path accepts
  multi-line; `cat seed | oceanln …` with one word per line fails.
- **Low** `ocean.rs:172` user-agent is `oceanln-mcp` for a shared client;
  `ocean.rs:111,114` make `Payout.ts` and `total_satoshis_net_paid` required so
  one malformed row rejects the whole `/earnpay` body.
- **Low** `tests/bip322_vectors.rs:1-21` credits the hashes to "the previous
  Zig implementation"; they are the BIP-322 spec vectors.
- **Nit** `random_token` lives in the signing module; `parse_bip32_path`
  accepts `m84'/…`; `payout` takes nine positional args.

## 4. Server, CLI, MCP, Tauri

Solid: constant-time bearer compare, bind refuses non-loopback, 2 MB body
limit via axum defaults, CORS only for configured origins, token printed once
to stderr, MCP is genuinely read-only (no POST path), Tauri CSP is
`script-src 'self'` with `connect-src` limited to IPC plus the two APIs,
capabilities grant only `core:default`.

- **High** B2 above.
- **Medium** `oceanln-httpd/src/main.rs:37-38,67-68`, `oceanln-mcp/src/main.rs:35-36`:
  `--token`, `--sidecar-credentials`, `--httpd-token` are argv, visible in
  `ps`. Add `--token-file` or `OCEANLN_TOKEN`.
- **Medium** `lib.rs:126` with `error.rs:26`, `sign.rs:123-131`: error bodies
  are `Error::to_string()`, leaking absolute paths, file modes and the sidecar
  URL to HTTP clients. Map to generic messages, log details server-side.
- **Medium** `src-tauri/src/lib.rs:227-239`: `open_external` opens any
  `http(s)://` URL via the `open` crate, bypassing the opener plugin scope.
  Allowlist `ocean.xyz` and `mempool.space`.
- **Medium** `oceanln-mcp/src/main.rs:69-71,91`: the MCP listener is
  unauthenticated and holds the httpd bearer, so any local process can read
  wallet address, offer and payout history even when httpd is in token mode.
  Document the downgrade or add `--mcp-token`.
- **Medium** `oceanln-cli/src/main.rs:470-521` duplicates `service::payout`
  including the offer-in-message check. Call the service.
- **Low** `lib.rs:434-455` duplicates `oceanln_common::net::host_is_loopback`
  while the comment says it is a wrapper. No request timeout or concurrency
  layer. `AppState::with_ocean_client` is public and unused, so `/ocean/*`
  success paths are untested. Tauri defaults `limit` to 200, HTTP to 100.
  `cli.rs:8-9` comment says `PayoutArgs` has no `Debug`; it does.
- **Low** `main.rs:479,488`: `--offer` validation is `starts_with("lno1")`
  plus substring; a truncated prefix passes.
- Tests missing: `/node`, `/activity`, `/invoice`, `/pay` success paths,
  `/ocean/*` success, `Origin: null`, IPv6 Host, oversized body, CLI `offer`
  and `payouts`, MCP `loopback_guard`.

## 5. Mobile (`mobile/`)

- **High** plain-file seed, no `FLAG_SECURE` or iOS blur (B7).
- **Medium** `mobile/core/src/lib.rs:83-85,106-110`: mnemonic crosses FFI as
  a plain `String`, not `Zeroizing`, and lands in JVM strings held in Compose
  state (`Onboarding.kt:61,65`) never cleared.
- **Medium** `CoreRepository.android.kt:49-63`: sync UniFFI calls doing
  PBKDF2 and file I/O run on `Dispatchers.Main`.
- **Medium** `CoreRepository.android.kt:55`, `CoreBridge.swift:128`: restore
  imports with `force=false` then provisions; a provisioning failure leaves
  the seed written and the retry 409s.
- **Medium** `mobile/core/src/lib.rs:286-335,680-790`: pool-stats derivation
  is a hand-port of `ocean.ts`/`StatsGrid.svelte`; move to
  `oceanln_common::ocean` so three frontends share it.
- **Low** `CoreRepository.ios.kt:76` discards the `confirmBackup` error;
  `lib.rs:393-394` exports `payout` hardwired to a localhost sidecar that
  cannot exist on a phone; `mobile/README.md` is stale in five places
  (`:154,172,205,220,262`); `mobile/design/README.md:6` links a private
  Claude Design project; versions `0.3.1`/`0.9.0`/`1.0` disagree.
- Tests present and meaningful: cache TTL/single-flight/invalidation, phrase
  paste, balance math. Nothing exercises the native adapters, which is
  expected.

## 6. Repository hygiene

Clean: no API keys, bearer values, node pubkeys, internal hosts or `.env` in
tracked files or history; only `demo-`/`dev-oceanln-local-token` placeholders;
all scripts use `set -euo pipefail`; the shared test mnemonic (`music mystery
… cream dune`) is a fixture; the AGPL relicense is sound because the only
human author is Vincenzo Palazzo (AI co-author trailers aside).

- **High** fonts: six Inter woff2 plus three Geist Mono ttf are byte-identical
  in `oceanln-web/public/fonts`, `oceanln-docs/static/fonts` and
  `docs/design/ocean-lightning-wizard/project/fonts`: about 3.4 MB of a 5.8 MB
  repo. Keep one copy. No OFL licence text is included anywhere; add
  `OFL-Inter.txt`, `OFL-Geist.txt` and `THIRD_PARTY_NOTICES.md`.
- **Medium** `docs/design/**`: two 25 KB AI design-chat transcripts,
  duplicated `Ocean Lightning Wizard.html`, `colors_and_type.css` ×4,
  `tweaks-panel.jsx` ×2, seven screenshots, a hidden `.thumbnail`. Drop the
  tree or keep one prototype set with a note.
- **Medium** OCEAN logos and `x.com/ocean_mining` footer, docs-site footer
  "© 2026 OCEAN. All rights reserved.", mobile bundle id `xyz.ocean`. State
  the affiliation and trademark permission in the README.
- **Medium** `oceanln-docs/src/lib/waitlist.svelte.ts:4-5,58-59`: a fabricated
  "214 of 500 spots remaining" counter and a "We'll email you" promise while
  emails only go to localStorage. Wire it or remove it.
- **Medium** `docs.yml` deploys to Cloudflare Pages on every push to `main`;
  gate on `github.repository == 'vincenzopalazzo/oceanln-cli'`. All actions
  pinned to floating major tags; `ci.yml`, `web.yml`, `mobile.yml` have no
  `permissions:` block; `ci.yml` double-runs on push and PR.
- **Medium** `oceanln-web/package.json` and `oceanln-docs/package.json` have
  no `license` field. Versions: workspace 0.3.0, Tauri/mobile/web 0.3.1.
- Missing: `SECURITY.md` (disclosure contact, plaintext-seed-at-rest note,
  mainnet warning for `web-demo.sh` and `lexe_e2e.sh`), `CONTRIBUTING.md`,
  `CODE_OF_CONDUCT.md`, `CHANGELOG.md`, `.github/dependabot.yml`, issue
  templates, `rust-toolchain.toml`, `cargo deny`/`cargo audit` job,
  `.editorconfig`.
- `docs/plans` and `docs/brainstorms`: useful history but phrased as AI
  session artefacts. Move under `docs/design-notes/` with an index and a
  "historical, may be stale" header.

---

## 7. Suggested order of work

1. B1: bump `bip322`, assert key/address match, add known-answer and negative tests.
2. B2: gate the seed- and key-touching routes under `--no-auth`.
3. B3, B4, B5: scrub the probe default, the skill file, the README and docs site.
4. B6: fix payout pagination.
5. B7: experimental banner on mobile, or branch it.
6. U1 and U2: recoverable backup and a visible "you signed exactly this" block. These two changes are what make the project a good BIP-322 example.
7. Hygiene: fonts, OFL texts, `docs/design`, SECURITY.md, CI permissions, package licences.
8. Then the medium and low items by crate.
