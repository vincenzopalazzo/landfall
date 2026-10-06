# QA scenario registry

The checklist every QA pass runs (`.claude/skills/qa-pass/SKILL.md`), and the
place that pass writes back to. It only grows: each bug a pass finds becomes a
scenario here, so the next pass checks it by default.

## Rules

1. **Every bug found in a QA pass adds or extends a scenario** in the same PR
   that fixes it: the reproduction, what "good" looks like, and the finding
   id (review item, issue, PR) under **Origin**.
2. **Automate what can be driven headlessly.** Prefer, in order: a unit or
   integration test at the real call site (named under **Guard**), then a
   scripted step in `scripts/qa/*.sh` or `oceanln-web/e2e/*.e2e.ts` (the
   scenario id appears in the PASS/FAIL line or the test title), then manual
   steps. Say which one applies under **How**.
3. **Keep ids stable.** Never renumber; retire a scenario by striking it
   through with the reason, so old QA reports still resolve.
4. **Every transport.** The same core sits behind the CLI, HTTP, the web
   wizard and Tauri IPC. A bug found on one is checked on the
   others; say which surfaces a scenario covers and why the rest are not
   driven.
5. **Money and seeds are manual.** Anything that provisions a real Lexe
   wallet, pays, or touches `~/.config/oceanln/seed` is a manual scenario
   run by a human with a throwaway wallet. The runners never do it.

The fixture phrase (`music mystery … cream dune`, address
`bc1qpstw48j7j9gjugw25jmjvd96jlwgdnedk5pr6r`) is a shared throwaway; never
fund it. "The wizard" is `oceanln-web` served from a QA bundle against
`oceanln-httpd --mock-wallet`.

## CLI

### QA-001 — `generate` prints a fresh phrase, once, cleanly
- **Surfaces:** CLI (automated)
- **Steps:** run `oceanln generate` twice; run with `--json`.
- **Expect:** 24 lowercase words on stdout, nothing else; the warning is on
  stderr; the two runs differ; `--json` is `{"mnemonic": …}` with 24 words.
- **How:** `cli-smoke.sh` QA-001, QA-002 · Guard: `smoke.rs::generate_prints_24_words`, `generate_json_has_24_word_mnemonic`

### QA-003 — `init --dry-run` derives without touching disk or network
- **Surfaces:** CLI (automated; default build only)
- **Steps:** pipe the fixture phrase into `init --dry-run --json --seed-file <new path>`.
- **Expect:** `mining_address` is the pinned address; the seed file does not exist afterwards.
- **How:** `cli-smoke.sh` QA-003 · Guard: `smoke.rs::init_dry_run_does_not_persist_seed_file`

### QA-004 — `payout --offer` signs offline and `verify` accepts it
- **Surfaces:** CLI (automated); HTTP (QA-109/110); wizard (QA-201/208)
- **Steps:** `payout --json --url http://127.0.0.1:1 --offer <lno1> --message "Configure OCEAN payout to <lno1> at block 840000"` with the phrase on stdin; then `verify --address --message --signature` with the printed values.
- **Expect:** no sidecar contact (the dead URL never matters); address is the pinned one; the offer is echoed; `verify` exits 0 with `valid: true`.
- **How:** `cli-smoke.sh` QA-004, QA-005 · Guard: `smoke.rs::verify_round_trips_payout_signature_and_rejects_tampering`, `sign.rs::signs_core_vector_key_and_round_trips`
- **Origin:** pre-release review B1 — with `bip322` 0.0.10 the verifier never bound the witness key to the address, so a wrong-key signature "verified".

### QA-006 — `verify` rejects everything that is not a signature by the address's key
- **Surfaces:** CLI (automated)
- **Steps:** take a valid triple; change one byte of the message; present it for a different `bc1q`; present garbage; present a legacy `1…` address.
- **Expect:** exit 1 each time with a reason, never a panic; `--json` has `valid: false` and `reason`.
- **How:** `cli-smoke.sh` QA-006 · Guard: `sign.rs::refuses_to_sign_for_address_key_does_not_control`, `verify_rejects_garbage_and_non_p2wpkh`, `bip322_vectors.rs::signature_for_wrong_address_fails_verify`
- **Origin:** review B1

### QA-007 — the offer must be embedded in the message
- **Surfaces:** CLI (automated); HTTP (QA-111); wizard (QA-204)
- **Steps:** `payout --offer X --message "… lno1other …"`.
- **Expect:** exit 1, "not present in --message"; nothing is signed.
- **How:** `cli-smoke.sh` QA-007 · Guard: `smoke.rs::payout_rejects_offer_not_in_message`

### QA-008 — `--path` changes the address and the signature follows the key
- **Surfaces:** CLI (automated)
- **Steps:** `payout … --path "m/84'/0'/0'/0/1"`.
- **Expect:** a different `bc1q` than the default path; `verify` accepts its signature for that address.
- **How:** `cli-smoke.sh` QA-008

### QA-009 — a loose seed file is refused
- **Surfaces:** CLI (automated); HTTP (startup validation)
- **Steps:** write the phrase to a file, `chmod 644`, pass it with `--seed-file`.
- **Expect:** exit 1 with a `chmod 600` hint; the file is not read.
- **How:** `cli-smoke.sh` QA-009

### QA-010 — usage errors exit 2
- **Surfaces:** CLI (automated)
- **Expect:** `--offer` with `--description` and a missing `--message` are clap errors (exit 2), distinct from runtime failures (exit 1).
- **How:** `cli-smoke.sh` QA-010 · Guard: `smoke.rs::payout_offer_conflicts_with_description`, `payout_without_message_is_usage_error`

### QA-011 — stdin seed with one word per line
- **Surfaces:** CLI (manual — known gap)
- **Steps:** `printf 'word\nword\n…' | oceanln payout …`.
- **Expect:** accepted like a seed file is; today it fails with "expected 24 words" because piped stdin reads one line (`sign.rs::resolve_seed`).
- **Origin:** review, core item 9. Open.

## HTTP server

### QA-101 — `/health` is the only open route
- **Surfaces:** HTTP (automated)
- **Expect:** `/health` 200 without a token; `/status` 401 without; 200 with.
- **How:** `httpd-smoke.sh` QA-101…103 · Guard: `server.rs::health_needs_no_auth`

### QA-104 — `/generate` reveals once and never overwrites
- **Surfaces:** HTTP (automated); wizard (QA-201)
- **Steps:** `POST /generate` twice.
- **Expect:** first: 24 words + `bc1q` address, seed file created `0600`; second: 409. `/status` then reports `configured: true` with that address.
- **How:** `httpd-smoke.sh` QA-104…106 · Guard: `server.rs::generate_*`

### QA-107 — `/seed/reveal` is explicit, authenticated and uncached
- **Surfaces:** HTTP (automated); wizard Profile → Reveal (manual)
- **Expect:** with the token: the stored phrase verbatim and `Cache-Control: no-store`; without: 401; under `--no-auth`: 403.
- **How:** `httpd-smoke.sh` QA-107, QA-108, QA-122 · Guard: `server.rs::seed_reveal_*`

### QA-109 — `/payout` signs offline and the CLI agrees
- **Surfaces:** HTTP + CLI (automated)
- **Steps:** `POST /payout {message, offer}`; feed `address`, `message`, `signature` to `oceanln verify`.
- **Expect:** 200; the address is the configured one; `verify` exits 0. An offer absent from the message is 400.
- **How:** `httpd-smoke.sh` QA-109…111

### QA-112 — wallet routes answer with the mock node
- **Surfaces:** HTTP (automated with `--features qa-mock`); real Lexe (manual, QA-301)
- **Expect:** `/init` → `provisioned: true`; `/offer` → `lno1…`; `/node`, `/payouts`, `/activity` → 200.
- **How:** `httpd-smoke.sh` QA-112…114 (SKIP without the feature)

### QA-115 — browser guards
- **Surfaces:** HTTP (automated)
- **Expect:** a non-allow-listed `Origin` is 403 even with a valid token; the allow-listed one is 200; its `OPTIONS` preflight advertises `POST` and `Authorization`; a non-loopback `Host` is 403.
- **How:** `httpd-smoke.sh` QA-115…118 · Guard: `server.rs` origin/host/CORS tests, `lib.rs::tests` for `host_is_loopback`

### QA-119 — `/import` replaces a wallet only when forced
- **Surfaces:** HTTP (automated); wizard (QA-210, open)
- **Expect:** a different phrase without `force` → 409; with `force: true` → 200 and `/status` shows the imported address.
- **How:** `httpd-smoke.sh` QA-119, QA-120 · Guard: `server.rs::import_*`

### QA-121 — `--no-auth` is read-only without a token
- **Surfaces:** HTTP (automated)
- **Steps:** start with `--no-auth`; `GET /status`; `POST` each of `/generate`, `/import {force:true}`, `/payout`, `/offer`, `/init`, `/invoice`, `/pay`, `/seed/reveal` with no header; `GET /status` again.
- **Expect:** the GET is 200; every POST is 403; the seed on disk is unchanged.
- **How:** `httpd-smoke.sh` QA-121…123 · Guard: `server.rs::no_auth_mode_refuses_every_seed_and_key_touching_route`, `no_auth_mode_serves_read_only_routes_without_bearer`
- **Origin:** pre-release review B2 — only `/pay` and `/seed/reveal` were gated; a local process could swap the seed or mint BIP-322 authorizations.

### QA-124 — a routable bind is refused
- **Surfaces:** HTTP (automated)
- **Expect:** `--bind 0.0.0.0:…` exits non-zero before serving.
- **How:** `httpd-smoke.sh` QA-124

## Web wizard (the BIP-322 UX)

### QA-201 — create a wallet, back it up, sign for OCEAN, hand off
- **Surfaces:** wizard (automated, Chromium); Tauri (manual, QA-401)
- **Steps:** Welcome → *Create a new wallet* → reveal → tick the backup box → Continue → answer the three quiz positions → Continue → wait for "Your wallet is ready" → Continue → paste `Configure OCEAN payout to <offer> at block 840000` → *Sign message* → Continue → *I've submitted these to OCEAN*.
- **Expect:** 24 words; `lno1…` offer and `bc1q…` address on the wallet step; the same two on the sign step; a signature longer than 80 characters; the hand-off screen shows all three; the final screen says payouts are on.
- **How:** `e2e/create.e2e.ts`; `web-e2e.sh` QA-208 then runs `oceanln verify` on the displayed triple and QA-209 re-derives the address from the revealed phrase.

### QA-202 — Continue is gated on reveal + backup
- **Expect:** Continue disabled until the blur is tapped AND the checkbox ticked.
- **How:** `e2e/create.e2e.ts` · Guard: `App.test.ts "gates Continue until the phrase is revealed and backed up"`

### QA-203 — a wrong quiz pick is flagged and blocks
- **Expect:** a wrong word shows "not the right word" and Continue stays disabled; the right word clears it.
- **How:** `e2e/create.e2e.ts`
- **Note:** four-option multiple choice is brute-forceable (review U3). Open: free-text entry.

### QA-204 — Sign is gated on the message embedding the offer
- **Expect:** *Sign message* disabled while the textarea is empty or names another offer; the "doesn't contain your offer" hint shows; enabled once the real offer is in the text.
- **How:** `e2e/create.e2e.ts`

### QA-205 — a relaunch finds the wallet
- **Steps:** after QA-201, reload.
- **Expect:** the profile surface (Profile / Lightning nav) with the payout address, not the Welcome step.
- **How:** `e2e/create.e2e.ts`

### QA-206 — import an existing phrase
- **Steps:** *I already have a phrase* → type the 24 fixture words → Continue.
- **Expect:** skips reveal/confirm; "Your wallet is ready" shows the pinned address.
- **How:** `e2e/import.e2e.ts`

### QA-207 — an invalid phrase is an error, not a dead end
- **Steps:** 24 × `abandon` → Continue.
- **Expect:** an error mentioning the mnemonic/checksum; the 24 boxes stay editable.
- **How:** `e2e/import.e2e.ts`

### QA-210 — close the tab before backing up (open)
- **Surfaces:** wizard (manual)
- **Steps:** *Create a new wallet*, do NOT reveal, close the tab, reopen.
- **Expect (desired):** an offer to reveal and back up the stored phrase. **Today:** the Import step demanding words the user never saw (`store.svelte.ts` bootstrap, "configured, no offer" branch).
- **Origin:** review U1. Open.

### QA-211 — import over an existing wallet (open)
- **Steps:** Create → Back → *I already have a phrase* → a different phrase → Continue.
- **Expect (desired):** a confirmed "replace the wallet on this server?" that retries with `force`. **Today:** a 409 error with no way forward.
- **Origin:** review U1. Open.

### QA-212 — typing the token in Settings must not reset an un-backed-up phrase (open)
- **Steps:** on the Phrase step, open Settings, type one character into the token field.
- **Expect (desired):** nothing changes until the field is committed. **Today:** every keystroke re-bootstraps and clears the phrase.
- **Origin:** review U1. Open.

### QA-213 — the sign step shows what was signed (open)
- **Expect (desired):** after signing, the exact message and the signing address are shown together and a "sign a different message" reset exists. **Today:** the textarea is disabled and the message is never echoed.
- **Origin:** review U2. Open.

## MCP proxy

### QA-301 — read-only, loopback, no `Origin`
- **Surfaces:** MCP (manual with any MCP client)
- **Steps:** run `oceanln-mcp --base <httpd> --httpd-token <t>`; list tools; call `get_status`, `list_payouts`; send a request with an `Origin` header.
- **Expect:** only read tools exist; results mirror httpd; the `Origin` request is refused.

## Desktop (Tauri)

### QA-401 — QA-201 through the desktop shell
- **Surfaces:** Tauri (manual)
- **Expect:** identical flow over IPC; the seed lands in the app-data dir `0600`; `ocean.xyz`/`mempool.space` links open in the OS browser.

## Mobile (experimental, on the `mobile-experimental` branch)

### ~~QA-501 — onboarding on an emulator~~
- Retired 2026-10-06: the mobile tree moved out of `main` to the
  `mobile-experimental` branch until it is production-hardened (plain-file
  seed, no screenshot protection, CI never built the native libraries).
  Re-open this scenario when it returns.

## Real wallet (manual, throwaway funds only)

### QA-601 — provision + offer + payout against Lexe
- **Steps:** `scripts/lexe_e2e.sh` with a fresh seed.
- **Expect:** `init` provisions; `offer` returns a payable `lno1`; OCEAN accepts the triple; `payouts` lists the first payout newest-first.
- **Origin:** review B6 — the listing used to return the oldest `limit` rows.
