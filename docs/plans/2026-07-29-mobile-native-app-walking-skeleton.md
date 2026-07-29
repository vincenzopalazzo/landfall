# Plan: mobile native app — walking skeleton (Rust core + Compose Multiplatform)

**Goal:** Stand up a new top-level `mobile/` folder implementing the OCEAN
Lightning mobile app bitkey-style — one shared **Rust** core (`oceanln-common`)
exposed to native mobile via **UniFFI**, with a **Compose Multiplatform** UI that
matches the "Ocean Lightning Mobile" design. This first increment is the walking
skeleton (brainstorm Approach A): prove the Rust→UniFFI→Compose pipeline with the
core fully built + verified and the UI laid down as reviewed source.

## Environment reality (probed 2026-07-29)

- ✅ cargo nightly 1.96; iOS + Android Rust targets installed; `uniffi-bindgen` present; Xcode 26.6; JDK 17.
- ❌ **No Gradle, `ANDROID_HOME` unset** → the Compose/Kotlin/Android layer **cannot be compiled in this environment**.

Consequence: the **Rust core is built and verified here** (`cargo check` + iOS
cross-compile attempt). The **Compose Multiplatform app ships as reviewed source
+ Gradle config**, compiled later on a machine with Gradle + Android SDK. This is
stated plainly in the PR — no skip-to-green.

## Affected files (new — nothing existing is modified except the workspace exclude)

- `Cargo.toml` (root) — add `mobile/core` to `[workspace].exclude` (like `src-tauri`).
- `mobile/core/Cargo.toml`, `src/lib.rs`, `src/ffi.rs`, `build.rs`, `uniffi.toml` — Rust UniFFI crate `oceanln-mobile-core`.
- `mobile/core/bin/uniffi-bindgen.rs` — bindings generator entry.
- `mobile/core/build-ios.sh`, `build-android.sh` — cross-compile + bindgen scripts.
- `mobile/composeApp/**` — Compose Multiplatform: theme (from `colors_and_type.css` tokens), app shell, screens, data layer.
- `mobile/androidApp/**`, `mobile/iosApp/**` — thin platform hosts.
- `mobile/design/**` — vendored design reference (jsx) + `colors_and_type.css` + assets.
- `mobile/README.md` — architecture + full build/run instructions + open questions.

## Approach

1. **`mobile/core` (Rust, UniFFI)** — an `OceanlnCore` object constructed with an
   app-data directory (holds the seed file via the existing `SeedSource`, exactly
   like the Tauri shell roots it in the OS app-data dir). Methods are thin
   adapters over `oceanln_common::service` + `wallet_provider::LexeWalletProvider`,
   mirroring `src-tauri/src/lib.rs` 1-1: `status`, `generate`, `import`,
   `reveal_seed`, `create_offer`, `init`, `payout`, `node_status`,
   `list_payments`, `list_offer_payouts`, `create_invoice`, `pay`. Async methods
   use `#[uniffi::export(async_runtime = "tokio")]`. Errors map to a
   `CoreError { status, message }` enum mirroring Tauri's `CommandError` (never
   carries the seed). DTOs (`NodeStatus`, `Activity`, `OceanPayout`, `StatusResp`,
   `PaySummary`) re-exposed as UniFFI records. Own `Cargo.lock`, excluded from the
   root workspace so `cargo --workspace` CI stays fast (same policy as `src-tauri`).
2. **Bindings** — proc-macro UniFFI (no UDL); `build-ios.sh` cross-compiles a
   static lib for `aarch64-apple-ios`(+sim), assembles an `.xcframework`, and runs
   `uniffi-bindgen` for Swift; `build-android.sh` builds the Android ABIs and runs
   bindgen for Kotlin. Generated bindings are git-ignored (regenerated at build).
3. **`mobile/composeApp` (Compose Multiplatform)** — `commonMain` theme
   (`Color.kt`, `Type.kt`, `Shapes.kt`) translated from the dashboard token block
   in `colors_and_type.css` (bg `#09090b`, card `#131316`, accent `#f7931a`, Inter
   + Geist Mono). App shell (header + 4-tab bar) and the **Pool** and **Wallet**
   screens implemented 1-1 (the walking-skeleton screens). A `WalletRepository`
   interface with `CoreWalletRepository` (calls the generated UniFFI bindings) and
   `MockWalletRepository` (design mock data) so the UI runs with or without a node.
   **Activity, Node, and the Send/Receive/TxDetail sheets are scaffolded as
   structured stubs** referencing the design — explicit follow-up increment.
4. **Hosts** — `androidApp` (MainActivity loads `.so` + Compose), `iosApp`
   (SwiftUI `ContentView` embeds the Compose `UIViewController` + `.xcframework`).
5. **Design sync** — vendor `colors_and_type.css`, the mobile `*.jsx` as visual
   reference, and OCEAN SVG assets into `mobile/design/`. Font binaries are
   fetched by a documented sync step (README) to avoid committing large blobs
   blind.

## Edge cases / risks

- **`lexe-sdk` cross-compile (the big open question):** the core depends on
  `oceanln-common` default features (in-process Lexe node). Its native tree may
  not cross-compile to iOS/Android. Mitigation: verify host build here; attempt
  the iOS target; if it fails, document the `--no-default-features` (thin
  sidecar) fallback. The UI's `MockWalletRepository` means the app is not blocked
  on this being resolved.
- **UniFFI async runtime:** service/wallet fns are `async`; must wire the tokio
  runtime in the export macro or expose blocking wrappers.
- **Seed on mobile:** walking skeleton uses the file `SeedSource` rooted at an
  app-provided dir. Keychain/Keystore-backed storage is a follow-up (noted).
- **CI:** root Cargo CI won't see `mobile/core` (excluded). A dedicated mobile CI
  workflow (macOS + Android SDK) is follow-up, mirroring how `src-tauri` is
  handled.

## Test plan

- `cd mobile/core && cargo check` and `cargo test` pass on host.
- `cargo build --target aarch64-apple-ios -p oceanln-mobile-core` attempted;
  result (pass or the specific lexe-sdk failure) reported honestly in the PR.
- `uniffi-bindgen` generates Kotlin + Swift without error.
- Kotlin/Compose: not compilable in this env — shipped as reviewed source; README
  documents the `./gradlew` build for a properly provisioned machine.

## Conventions to follow

- Mirror `src-tauri/src/lib.rs`: thin adapters over `service`, `CommandError`-style
  mapping via `service::http_status`, seed loaded per-call and never returned
  except explicit `reveal`.
- Exclude the heavy-native subproject from the root workspace (`src-tauri` policy).
- AGPL-3.0 license headers consistent with the repo.

**Estimated size:** L (new subsystem; core is the verified slice).
