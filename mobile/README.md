# OCEAN Lightning — Mobile (iOS + Android)

Native mobile app built the [bitkey way](https://engineering.block.xyz/blog/how-bitkey-uses-cross-platform-development):
**one shared core, native-rendered UI on every platform.** Here the shared core
is the existing **Rust** `oceanln-common` crate — the same audited flow behind
the CLI, `oceanln-httpd`, the Tauri desktop shell, and the MCP server — exposed
to mobile over **UniFFI**, with a **Compose Multiplatform** UI that renders
natively on both iOS and Android from a single Kotlin codebase.

```
mobile/
├── core/            Rust: UniFFI wrapper over oceanln_common::service (+ wallet)
│                    → generated Kotlin (Android) + Swift (iOS) bindings + native libs
├── composeApp/      Compose Multiplatform UI (Kotlin) — the 1-1 design impl
│   ├── commonMain/  theme, data, ui, screens, sheets  (shared, all platforms)
│   ├── androidMain/ MainActivity + Android core binding (JNA) + jniLibs
│   └── iosMain/     MainViewController (Compose → UIViewController)
├── iosApp/          SwiftUI host embedding the Compose framework + core xcframework
├── design/          vendored design tokens/assets (see design/README.md)
└── README.md
```

## Architecture

The UI never talks to a node or the network directly — it renders against a
`WalletRepository` (`composeApp/.../data/WalletRepository.kt`). Two impls:

- `MockWalletRepository` — the design's mock data; the app runs 1-1 with the
  handoff with no node.
- `CoreWalletRepository` — live data from the Rust core via UniFFI, provided per
  platform by `createCoreRepository` (`expect`/`actual`).

The Rust `OceanlnCore` object (`core/src/lib.rs`) is a thin adapter over
`oceanln_common::service` — a 1-1 twin of `src-tauri/src/lib.rs`. The seed stays
on device (read locally per call, signing keys wiped after use); it crosses the
FFI boundary only on an explicit `reveal_seed`.

## Build

### 1. Rust core → bindings + native libs

```sh
cd mobile/core
cargo check                 # host sanity (also builds the full Lexe SDK tree)
./build-ios.sh              # → generated/OceanlnMobileCore.xcframework + Swift glue
./build-android.sh          # → composeApp jniLibs/*.so + generated Kotlin (androidMain)
```

`build-android.sh` needs `cargo-ndk` (`cargo install cargo-ndk`) + the Android NDK.

### 2. Android app

```sh
cd mobile
gradle wrapper             # once, to fetch the wrapper jar (no wrapper jar is committed)
./gradlew :composeApp:assembleDebug
./gradlew :composeApp:installDebug   # onto a running emulator/device
```

`ANDROID_HOME` must point at the SDK (add a `local.properties` with `sdk.dir=…`).

### 3. iOS app

Open `iosApp/` in Xcode, add the `ComposeApp` framework (from
`./gradlew :composeApp:embedAndSignAppleFrameworkForXcode`) and
`core/generated/OceanlnMobileCore.xcframework`, then Run. The `.xcodeproj` is not
committed — create it with the standard Compose Multiplatform template or add the
two `iosApp/iosApp/*.swift` sources + `Info.plist` to a new SwiftUI app target.

## Status (this increment = walking skeleton)

Verified here (2026-07-29, this machine):

- ✅ `mobile/core` `cargo check` + `cargo build` on host.
- ✅ **`cargo build --target aarch64-apple-ios` succeeds** — the full Lexe SDK
  (in-process node) cross-compiles to iOS with default features. This resolves
  the brainstorm's biggest open risk: no thin-surface fallback is needed on iOS.
- ✅ UniFFI `generate` emits Kotlin + Swift bindings for all 12 core methods.

Laid down as reviewed source, **not** compiled here (this box has no Gradle /
Android SDK): the Compose Multiplatform app, the Android/iOS hosts, Gradle
config. Build them on a provisioned machine per the steps above.

Screens implemented 1-1: **Pool**, **Wallet** (simple + full). **Activity**,
**Node**, and the **Send / Receive / Transaction-detail** sheets are implemented
and wired into the shell; polish + real-data wiring continues in follow-ups.

## Open follow-ups

- **iOS core binding.** The Rust core builds for iOS and UniFFI emits Swift, but
  bridging that into the Compose (Kotlin/Native) data layer isn't wired — iOS
  runs on mock data today. Preferred fix: KMP-native UniFFI bindings via
  [gobley](https://github.com/gobley/gobley) so one Kotlin binding serves both
  JVM and Native; alternative: a thin Swift↔Kotlin shim.
- **Seed storage.** The core uses a file `SeedSource` under the app-data dir.
  Move to iOS Keychain / Android Keystore (a new `SeedSource` variant — no
  call-site changes).
- **Pool/worker stats.** Come from the ocean.xyz pool API, not the node; still
  mock in `CoreWalletRepository`.
- **Fonts.** Vendor Inter + Geist Mono into compose resources (see design/README).
- **CI.** Add a mobile workflow (macOS + Android SDK); the root `cargo --workspace`
  CI excludes `mobile/core` by design.
```
