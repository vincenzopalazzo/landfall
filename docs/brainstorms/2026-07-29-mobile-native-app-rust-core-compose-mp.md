# Mobile app — native, Rust core shared, Compose Multiplatform UI

## Clarified Problem Statement

**Goal:** Build a new top-level `mobile/` folder containing a native iOS + Android
app that reuses the existing Rust core (`oceanln-common`) as its shared business
logic — bitkey-style cross-platform architecture — and implements the
`Ocean Lightning Mobile` design 1-1 as its UI.

**Settled decisions (from clarification):**
- **Architecture** — bitkey model: one shared core, native-rendered UI. Applied
  here with the core pinned to **Rust** (`oceanln-common`), not KMP.
- **Shared logic seam** — the core is exposed to mobile via **UniFFI**, wrapping
  the transport-agnostic `oceanln_common::service` functions (`generate`,
  `import`, `status`, `offer`, `init`, `payout`, `reveal`) plus the
  `lexe_wallet` / `wallet_provider` reads (node status, activity, payouts) and
  sends (`pay`, `createInvoice`). Same audited flow the CLI / httpd / Tauri /
  MCP frontends already call — mobile becomes a fifth frontend over the one core.
- **UI** — **Compose Multiplatform** (Kotlin), one UI codebase rendering natively
  on both iOS and Android. This is what the bitkey article says they migrated
  *to* (away from separate SwiftUI + Jetpack Compose).
- **"Sync the design locally"** — pull the design project's mobile assets into
  this repo via DesignSync (the JSX as visual reference, `colors_and_type.css`
  tokens, Inter + Geist Mono fonts, OCEAN SVG/PNG assets) so the app builds and
  renders offline.

**Constraints:**
- Reuse `oceanln-common` as-is; don't fork the security-critical flow. The seed
  must never cross an FFI boundary except the one-time reveal, mirroring how
  `service.rs` already treats it.
- `mobile/` gets its own build system (Gradle + Kotlin Multiplatform) and is
  **excluded from the root Cargo workspace**, exactly like `src-tauri/` is today,
  so `cargo --workspace` CI stays fast.
- The Rust FFI crate builds to the mobile targets: `aarch64-apple-ios` (+ sim)
  static framework for iOS, and `arm64-v8a`/`x86_64` `.so` for Android.
- Match the design's dark dashboard surface + Bitcoin-orange accent
  (`[data-theme="dashboard"]` token block) pixel-for-pixel via a Compose theme
  derived from `colors_and_type.css`.
- `lexe-sdk` is a default feature of `oceanln-common` with a heavy native dep
  tree; confirm it cross-compiles to iOS/Android, or gate mobile to the thin
  (`--no-default-features`) surface + sidecar client where it doesn't.

**Non-goals:**
- Not porting the Svelte `oceanln-web` wizard or the Tauri desktop shell.
- Not a React Native / Flutter / web-view wrapper (rejected — not native).
- Not a Rust-native UI (Dioxus/Slint) — rejected in favor of Compose MP.
- Not shipping to the App Store / Play Store in this pass (device build + run is
  enough).

**Success criteria:**
- `mobile/` builds an installable iOS app and Android app from one UI codebase.
- The four screens (Pool/Mining, Wallet simple+full, Activity, Node/Settings)
  and the Send / Receive / TxDetail sheets render 1-1 with the design.
- At least the Pool + Wallet screens read **real** data through the Rust core via
  UniFFI (node status, balances, activity/payouts), proving the full
  Rust → UniFFI → Kotlin → Compose → iOS/Android pipeline.
- Design tokens, fonts, and assets are vendored into the repo; no CDN/runtime
  dependency on the Claude Design project.

## Target `mobile/` layout

```
mobile/
  core/                 # Rust: UniFFI wrapper over oceanln-common::service + lexe_wallet
    Cargo.toml          #   crate-type = ["staticlib","cdylib"]; depends on oceanln-common
    src/lib.rs          #   #[uniffi::export] async fns; UDL or proc-macro bindings
    build.rs            #   uniffi-bindgen (Kotlin)
    uniffi.toml
  shared-ui/            # Compose Multiplatform (Kotlin) — the 1-1 design impl
    theme/              #   Color/Type/Shape from colors_and_type.css tokens
    screens/            #   Pool, Wallet(simple/full), Activity, Node
    sheets/             #   Send (dest→amount→review→confirm→success), Receive, TxDetail
    components/         #   TxRow, Pill, maturity bar, hero, stat grid, tab bar
  androidApp/           # thin Android host (Activity + .so + generated Kotlin)
  iosApp/               # thin SwiftUI host embedding the Compose framework + core.xcframework
  fonts/  assets/       # vendored Inter, Geist Mono, OCEAN SVG/PNG
  README.md             # build/run for both platforms
```

## Approaches Considered

The UI toolkit and core seam are settled. These three differ on **how to
sequence and de-risk** the first deliverable.

### Approach A: Walking skeleton first (recommended)
- Sketch: Stand up `mobile/core` UniFFI crate exposing 2-3 real calls
  (`status`, `nodeStatus`, `activity`), generate Kotlin bindings, get one Compose
  screen (Wallet or Pool) rendering real data, and boot it on **both** an iOS
  simulator and an Android emulator. Only then build out the remaining screens +
  sheets against the now-proven pipeline.
- Affected files: new `mobile/core/*`, `mobile/shared-ui/*`, `mobile/androidApp`,
  `mobile/iosApp`; `oceanln-common` untouched (consumed as a path dep).
- Tradeoffs: De-risks the riskiest parts up front — UniFFI async export, the
  iOS `xcframework` + Android `.so` build wiring, and cross-compiling
  `lexe-sdk`'s native tree. Slower to a "looks complete" screenshot.
- Effort: L (but front-loads the unknowns).

### Approach B: UI-first on mock data, wire core after
- Sketch: Build all four Compose MP screens + sheets 1-1 using the design's mock
  data (POOL/WORKERS/BAL/TX ported to Kotlin), get a pixel-perfect running app on
  both platforms, then replace the mock repository with the UniFFI core.
- Affected files: `mobile/shared-ui/*` first; `mobile/core/*` second.
- Tradeoffs: Fast to a demoable 1-1 UI; parallelizes design fidelity from FFI
  plumbing. Risk: the FFI/build integration (the hard part) is deferred, and a
  late surprise (e.g. `lexe-sdk` won't cross-compile) could force rework of the
  data layer.
- Effort: M to first demo, L overall.

### Approach C: Full vertical slice in one pass
- Sketch: Build core FFI + all screens + both hosts + real data end-to-end before
  showing anything.
- Affected files: everything under `mobile/` at once.
- Tradeoffs: No intermediate checkpoints; highest chance of a large,
  hard-to-debug integration at the end. Not recommended for a first pass on an
  unproven toolchain.
- Effort: XL.

## Recommendation

**Approach A (walking skeleton).** The genuine risk here isn't the UI — Compose
can match the design — it's the toolchain: UniFFI async exports, embedding a
Compose framework in an iOS host, and whether `oceanln-common`'s default
`lexe-sdk` native dependency tree cross-compiles to `aarch64-apple-ios` and
Android ABIs. Proving one real screen on both platforms first turns those from
end-of-project surprises into day-one findings, then the remaining screens are
straightforward fills against a known-good seam.

## Open questions

- Does `oceanln-common` with default `lexe-sdk` cross-compile to iOS + Android
  targets? If not, mobile likely uses `--no-default-features` (thin sidecar
  client) and talks to a Lexe node the same way the sidecar path does — needs a
  quick spike.
- UniFFI async: the `service`/`wallet_provider` calls are `async`. Confirm the
  UniFFI async-runtime support (tokio) is wired, or expose blocking wrappers.
- Seed handling on mobile: where does the mnemonic live (iOS Keychain / Android
  Keystore)? The core reads via `SeedSource`; the FFI layer must supply a
  mobile-appropriate implementation rather than the CLI/file source.
- iOS build in CI: the root CI is Cargo-only; a mobile build needs macOS +
  Xcode + Android SDK runners — probably a separate workflow, mirroring how
  `src-tauri` is handled.
