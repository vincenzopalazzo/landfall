# Brainstorm: OCEAN Lightning desktop app (Tauri, native IPC)

## Clarified Problem Statement

**Goal:** Ship a runnable Tauri desktop shell for the existing `oceanln-web` wizard
that talks to the Rust backend over native Tauri IPC commands (no loopback HTTP
server, no bearer token), with the seed stored in the OS app-data dir.

**Decisions locked (from Q&A):**
- **Transport:** Native Tauri IPC (`#[tauri::command]` + `invoke()`). The desktop
  build has *no* `oceanln-httpd` server, *no* loopback port, and *no* bearer token —
  the smallest possible attack surface. The browser/server build keeps HTTP.
- **Scope this pass:** A working dev shell on the current OS (macOS) — `cargo tauri dev`
  opens a window, the wizard runs end-to-end, the seed lands in app-data. Installers,
  code-signing, and the cross-OS CI matrix are explicitly later.
- **Seed:** OS app-data dir (`~/Library/Application Support/oceanln/seed`, `0600`),
  resolved via Tauri's path API. No token (native IPC), so the wizard's token-paste /
  connection settings are hidden under Tauri.

**Constraints / must-not-break:**
- The shared signing/seed security invariants stay intact: seed never leaves the
  process, `MnemonicSecret` zeroizing, owner-only `0600` perms, atomic `O_EXCL` seed
  create, offer-must-be-in-message check, key wiped after signing.
- The browser build (`oceanln-httpd` + `fetch`) must keep working unchanged — the web
  app gains a transport abstraction, it does not lose the HTTP path.
- The public OCEAN API client (`ocean.ts`, dashboard) stays plain HTTPS to
  `api.ocean.xyz` — unaffected by the IPC change.
- Don't regress the Rust CI: adding a heavy Tauri crate to the workspace must not slow
  or break the existing `check`/thin-build/CLI jobs (likely exclude `src-tauri` from the
  default CI build for now).

**Non-goals (this pass):**
- Packaged/signed installers (`.dmg`/`.msi`/`.AppImage`), auto-update, CI build matrix.
- Windows/Linux bring-up (only the current OS needs to run).
- Replacing the browser/HTTP transport — it stays for web deployments.
- Any new wallet feature; this is purely a new shell over the existing flow.

**Success criteria:**
- `cargo tauri dev` opens a native window running the wizard; create + import + offer +
  init + payout(sign) all work via `invoke()` with no HTTP server running.
- No bearer token anywhere in the desktop path; Settings hides the connection fields
  under Tauri.
- Seed is written to the app-data dir at `0600`; relaunch reuses it.
- `oceanln-web` browser build + tests still pass unchanged; Rust CI stays green.

## Approaches Considered

### Approach A: In-process httpd (loopback) — *rejected*
- Sketch: Tauri main starts `oceanln_httpd` on an ephemeral 127.0.0.1 port with a minted
  token, injects base+token into the page; `api.ts` stays pure `fetch`.
- Tradeoffs: zero web-side churn, but keeps a loopback HTTP surface + token plumbing
  inside the app for no benefit when we own both sides of the IPC.
- Effort: M.

### Approach B: Sidecar httpd binary — *rejected*
- Sketch: bundle the compiled `oceanln-httpd` binary, Tauri spawns it as a sidecar and
  manages its lifecycle.
- Tradeoffs: most moving parts (process lifecycle, port, token), worst fit since we
  control the Rust.
- Effort: M.

### Approach C: Native Tauri IPC commands — *chosen*
- Sketch: New `src-tauri/` crate. Extract the orchestration currently in
  `oceanln-httpd/src/lib.rs` handlers into a transport-agnostic **service layer**
  (shared by axum and Tauri). Expose `generate / import / init / offer / payout` as
  `#[tauri::command]`s that call the service with a `SeedSource::File(app_data/seed)`.
  The web app gets a `Backend` interface with two impls — `HttpBackend` (existing
  `fetch`) and `TauriBackend` (`invoke`) — selected at runtime by detecting Tauri.
- Affected files/modules:
  - **new** `src-tauri/` (Cargo member): `tauri.conf.json`, `src/main.rs`, `src/lib.rs`
    (the commands), `build.rs`, icons.
  - **new/refactor** service layer: lift handler bodies out of
    `oceanln-httpd/src/lib.rs` into reusable functions (new `service` module in
    `oceanln-httpd` or a small `oceanln-core`), consumed by both the axum handlers and
    the Tauri commands. Keeps the offer-in-message check, seed-load, derive, sign, wipe,
    atomic store in one place.
  - **refactor** `oceanln-web/src/lib/api.ts`: introduce the `Backend` interface +
    `HttpBackend`/`TauriBackend`; `store.svelte.ts#client()` returns the right one.
  - `oceanln-web/src/lib/Settings.svelte` / `config.ts`: hide token+base under Tauri.
  - `oceanln-web/package.json`: add `@tauri-apps/api` (+ `@tauri-apps/cli` dev).
  - root `Cargo.toml`: add `src-tauri` member (and likely exclude it from the default CI
    build for now).
- Tradeoffs: **gains** the strongest security posture (no port, no token, no CORS/Origin/
  Host surface) and a single process; **costs** a Rust service-layer refactor + a web
  transport abstraction, and diverges the desktop transport from the browser (mitigated
  by sharing the service layer and the `Backend` interface). Tauri pulls a large native
  dependency tree.
- Effort: L.

## Recommendation

Proceed with **Approach C** (the user's pick). The decisive move that keeps it clean is
the **service-layer extraction**: pull the handler orchestration out of the axum layer so
the Tauri commands and the HTTP handlers are two thin adapters over one audited core —
this prevents the desktop and browser paths from drifting on the security-critical logic.
On the web side, a small `Backend` interface (`HttpBackend` vs `TauriBackend`) localizes
the divergence to one file. Keep `src-tauri` out of the existing Rust CI build this pass
to protect build times; wire its own job when we do installers.

## Open questions

- **Service-layer home:** a `service` module inside `oceanln-httpd`, or a new
  `oceanln-core` crate? Leaning `oceanln-httpd::service` to avoid a new crate now; revisit
  if `src-tauri` shouldn't depend on the httpd crate.
- **Tauri detection in the web app:** check `'__TAURI_INTERNALS__' in window` (Tauri v2) at
  startup and pick the backend — confirm Tauri v2 (latest) is the target.
- **CI:** exclude `src-tauri` from `cargo build --release --workspace` (e.g. workspace
  `exclude` or a default-members list) so the existing job stays fast — confirm acceptable.
- **lexe-sdk:** `src-tauri` needs `oceanln-common`'s `lexe-sdk` feature (real offer/init).
  Confirm the desktop app always builds with it (no thin desktop variant).
- **Toolchain:** `cargo tauri` CLI + platform webview must be present locally; macOS uses
  the built-in WKWebView (no extra install). Confirm `cargo install tauri-cli` is OK here.
