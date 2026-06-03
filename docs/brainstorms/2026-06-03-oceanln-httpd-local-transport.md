# Brainstorm: local HTTP transport for a web/desktop frontend

Date: 2026-06-03

## Clarified Problem Statement

**Goal:** Expose `oceanln`'s operations (`generate` / `init` / `offer` / `payout`) over a
single local channel that both a browser web app and a desktop shell can call, without the
BIP39 seed ever crossing that channel. Package it as a Cargo workspace following the
[`vincenzopalazzo/ocean-ln`](https://github.com/vincenzopalazzo/ocean-ln) pattern.

**Constraints:**
- One transport serves both a real browser and a desktop shell → loopback HTTP/JSON
  (rules out raw unix socket / stdio, which a browser cannot speak).
- Seed stays inside the process. The frontend sends only the OCEAN `message` + params
  (description, min_amount, path). `oceanln-common` obtains the mnemonic from its own
  local source via a `SeedSource` trait (file path / OS keychain / one-time unlock),
  signs, zeroizes, and returns. The mnemonic is never a request or response field.
- Reuse existing logic (`sign.rs`, `client.rs`, `lexe_wallet.rs`); transport is a thin
  shell over it.
- Loopback only (`127.0.0.1`). Because a browser is in scope, DNS-rebinding and CSRF are
  real threats → strict `Origin`/`Host` allowlist + bearer token + restricted CORS.
- `--no-default-features` (or building only `oceanln-cli`) still yields the plain CLI;
  the server and its deps are isolated in their own crate.

**Non-goals:**
- No long-running warm Lexe node (thin transport: each request is cold, like the CLI).
- No remote/LAN exposure, no auth server, no multi-user.
- No change to signing or wallet logic.
- CLI does NOT route through the server — it keeps its offline/in-process path.

**Success criteria:**
- A browser `fetch('http://127.0.0.1:<port>/payout', …)` and a desktop shell both get
  back `{address, offer, signature}` for the same request.
- No request/response body ever contains the raw mnemonic.
- A page from an untrusted origin cannot drive a payout (origin/token guard rejects it).
- Building `oceanln-cli` alone does not pull in axum/web deps.

## Chosen approach: Cargo workspace, axum loopback server (ocean-ln pattern)

Mirrors `ocean-ln`'s workspace conventions, adapted to this repo:

```
Cargo.toml                 # [workspace] resolver = "2", members + default-members
oceanln-common/            # shared: sign, client, error, SeedSource trait, Lexe backend
                           #   (lexe-sdk optional feature lives here)
oceanln-httpd/            # axum loopback server (127.0.0.1) — web app / UI runs this
oceanln-cli/               # clap binary — for AI/CLI users; keeps offline in-process path
```

File moves from today's single crate:
- `src/sign.rs`, `src/client.rs`, `src/error.rs` → `oceanln-common/src/`
- `src/lexe_wallet.rs` → `oceanln-common/src/` (behind `lexe-sdk` feature)
- `src/cli.rs`, `src/main.rs` → `oceanln-cli/src/`
- new `oceanln-httpd/src/main.rs` (axum app: routes `POST /generate|/init|/offer|/payout`)
- `tests/` → split: BIP-322 vectors + sign tests follow `oceanln-common`; CLI smoke
  tests follow `oceanln-cli`; add httpd integration tests under `oceanln-httpd/tests/`.

**Relationship:** `oceanln-cli` and `oceanln-httpd` are both thin frontends over
`oceanln-common`; neither depends on the other. The web/UI runs the httpd; the CLI is for
AI/manual use. (Diverges from ocean-ln, where the CLI is a network client of the httpd —
intentional, to preserve this repo's offline CLI.)

**Stack:** axum (not Actix Web) — already a dev-dep here, async/tokio is already in use,
lighter footprint. Add `tower-http` for CORS. Documented divergence from ocean-ln's
`oceanln-httpd`, which uses Actix Web.

**Security must-haves (browser in scope):** bind `127.0.0.1` explicitly; `Origin`/`Host`
allowlist; `Authorization: Bearer <token>` on every seed-touching route (token printed on
startup / delivered out-of-band to the web app); CORS restricted to known app origin(s).

**Effort:** M (workspace split is mechanical; axum server + guards is the new code).

## Open questions

- **Seed source:** which `SeedSource` impls ship first — configured file path, OS keychain,
  or one-time unlock held in memory? (The last quietly makes httpd a warm daemon — confirm.)
- **Token delivery to the browser:** env injection at build, a localhost pairing screen, or
  copy-paste? Blocks a usable browser flow, not the design.
- **Web app origin:** served from `localhost`, `file://`, or a hosted origin? Sets the exact
  CORS allowlist.
- Whether to split a separate `oceanln-backends` crate now or keep the Lexe backend inside
  `oceanln-common` behind the `lexe-sdk` feature (current plan: keep it in common).
