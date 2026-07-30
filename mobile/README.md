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

### The iOS binding (Swift ↔ Kotlin)

Kotlin/Native cannot call UniFFI's generated **Swift** API, so the dependency is
inverted rather than reimplemented:

1. Kotlin declares `WalletCoreBridge`
   (`composeApp/src/iosMain/.../data/WalletCoreBridge.kt`), which the ComposeApp
   framework exports as an Objective-C protocol.
2. Swift implements it in `iosApp/iosApp/CoreBridge.swift`, calling the generated
   `OceanlnCore` directly.
3. `MainViewController(bridge:)` receives it; `CoreRepository.ios.kt` converts
   the bridge's completion handlers back into `suspend` functions, so the shared
   UI sees the same `WalletRepository` as Android.

The bridge is deliberately callback-based (Kotlin `suspend` in an exported
interface is awkward to conform to from Swift) and speaks signed types only
(Kotlin/Native does not export unsigned), with `-1` as the "absent" sentinel
where Objective-C has no optional `Int64`.

The UI never talks to a node or the network directly — it renders against a
`WalletRepository` (`composeApp/.../data/WalletRepository.kt`). There is exactly
one implementation, `CoreWalletRepository`, backed by the Rust core via UniFFI
and provided per platform by `createCoreRepository` (`expect`/`actual`).

**There is no mock/fixture repository, by design.** An app that moves real funds
must not be able to render a plausible fake balance, so a build that cannot
reach the core shows an error instead (`MainActivity` / `MainViewController`).
Every figure on screen has a real source:

| Screen data | Source |
|---|---|
| Balances, channels, node identity | `lexe_wallet::node_status` (in-process node) |
| Activity / payments | `lexe_wallet::list_payments` |
| Hashrate, workers, shares, unpaid, lifetime paid | `ocean::OceanClient` → `api.ocean.xyz/v1` |
| BTC/USD for the fiat toggle | `price::PriceClient` → mempool.space |
| Payment fees | the node, at send time — never estimated in the UI |

The pool figures are derived exactly as `oceanln-web/src/lib/StatsGrid.svelte`
derives them, so mobile and the web dashboard cannot drift. Where OCEAN's public
API has no field — pool-wide hashrate, blocks found, last block, reject rate, a
per-worker list — the UI omits it rather than inventing a number.

A missing BTC/USD rate is never a fabricated one: `btc_usd()` returns `0`, the
sats/USD toggle hides itself, and every amount renders in sats.

### Caching

Every screen loads through `produceState`, which re-runs when the screen enters
composition — so without a cache one lap of the tab bar cost **8 HTTP requests
to ocean.xyz and 4 `nodeStatus()` calls**. `CachedWalletRepository` wraps the
platform repository (`.cached()`, applied at both entry points) and fixes that.

Three things about it matter more than the caching:

- **TTLs are per-entry, matched to volatility.** Node balances 15s (money — a
  payment landing while you're on another tab shows within 15s of returning);
  activity 20s; the pool snapshot 60s (four HTTP calls behind one entry, against
  an API that only samples periodically); seed-derived state until invalidated.
  One global TTL would either hammer ocean.xyz or show a stale balance.
- **Mutations invalidate.** `pay()` drops the balance and activity entries,
  `createInvoice()` drops activity, the seed operations drop everything. Serving
  a pre-send balance is the one bug this layer must not introduce.
- **Single-flight.** Concurrent callers of the same key share one in-flight
  request, and the load is owned by the repository's own scope so leaving a tab
  mid-fetch doesn't cancel it — otherwise coming back would refetch, which is
  exactly the behaviour being removed.

Never cached: `pay`, `createInvoice`, the seed operations, `revealSeed` (a cached
reveal would hold the phrase in memory past the one call that needs it), and
`payableAmountSats`. Retry buttons call `refresh()` to force a refetch.

Several interface methods are *derived defaults* rather than their own calls:
`offer()`/`miningAddress()`/`isWalletConfigured()` read one cached `status()`,
and `balances()` reads one cached `nodeInfo()`. That collapsed a duplicate
`nodeStatus()` round-trip that existed even before caching.

On first launch, the Compose shell checks the core for a configured seed and
shows the OCEAN onboarding flow when none exists. Creating a wallet generates
and displays the 24-word recovery phrase for an explicit backup check; restoring
accepts a 24-word phrase (including pasting the full phrase into the first
field). Backup confirmation is persisted separately from the seed, so an
interrupted create flow resumes at the phrase screen. Completion provisions the
wallet and creates its initial offer before entering the app. Both operations
use the same `oceanln-common` service path as the CLI.
The live Receive and Send sheets likewise obtain invoices, addresses, offers,
and payment results from that repository; unavailable destinations are never
substituted with design fixtures. Fixed-amount BOLT11 invoices lock the amount
shown during review to the value encoded by the invoice.

The Rust `OceanlnCore` object (`core/src/lib.rs`) is a thin adapter over
`oceanln_common::service` — a 1-1 twin of `src-tauri/src/lib.rs`. The seed stays
on device (read locally per call, signing keys wiped after use); it crosses the
FFI boundary only on an explicit `reveal_seed`.

## Build

### 1. Rust core → bindings + native libs

```sh
cd mobile/core
cargo check                 # host sanity (also builds the full Lexe SDK tree)
./build-ios.sh              # → generated/swift/* + the per-target static libs
./build-android.sh          # → composeApp jniLibs/*.so + generated Kotlin (androidMain)
```

`build-ios.sh` is required before the iOS app builds: it emits
`generated/swift/OceanlnMobileCore.swift` (+ the FFI header and
`module.modulemap`) and the `liboceanln_mobile_core.a` archives the Xcode target
links. For a simulator-only loop that is just:

```sh
cargo build --release --target aarch64-apple-ios-sim --lib
```

`build-android.sh` needs `cargo-ndk` (`cargo install cargo-ndk`) + the Android NDK.

### 2. Android app

```sh
cd mobile
./gradlew :composeApp:assembleDebug
./gradlew :composeApp:installDebug   # onto a running emulator/device
```

`ANDROID_HOME` must point at the SDK (add a `local.properties` with `sdk.dir=…`).

### 3. iOS app (simulator)

The Xcode project is generated from `iosApp/project.yml` with
[xcodegen](https://github.com/yonyz/xcodegen) (`brew install xcodegen`); it wires
a build phase that runs `:composeApp:embedAndSignAppleFrameworkForXcode`, so no
manual framework linking. Requires **JDK 17** (`JAVA_HOME`) and Gradle via the
committed wrapper.

```sh
cd mobile/iosApp
xcodegen generate
SIM=$(xcrun simctl list devices available | grep -m1 'iPhone 16 Pro' | grep -oE '[0-9A-F-]{36}')
open -a Simulator
xcodebuild -project iosApp.xcodeproj -scheme iosApp -configuration Debug \
  -sdk iphonesimulator -destination "platform=iOS Simulator,id=$SIM" \
  -derivedDataPath build build
xcrun simctl install booted build/Build/Products/Debug-iphonesimulator/iosApp.app
xcrun simctl launch booted xyz.ocean.mobile
```

Requires the Rust core built for the simulator first (step 1) — `project.yml`
links `-loceanln_mobile_core` from `core/target/<triple>/release` and puts the
generated Swift glue on the compile path.

Verified running on the iPhone 16 Pro simulator (Xcode 26): real seed generation,
a real BOLT12 offer + BOLT11 invoice + BIP84 address from the live node, and live
pool stats from ocean.xyz.

### Handling a wallet's failure modes

A few conventions this code holds to, because the usual defaults are wrong when
the data is someone's money:

- **Never advise a reinstall.** The seed lives in the app's private container;
  reinstalling deletes it. Error screens say the wallet is unchanged and point
  at the recovery phrase instead.
- **The backup marker never outlives its seed.** `generate`/`import_seed` clear
  it, so a replaced seed can't inherit "the user already wrote this down" and
  skip the phrase screen.
- **Errors are never flattened into values.** The Swift bridge cannot `throw`
  across the Objective-C export, so every sync call carries an `error` field the
  Kotlin adapter rethrows. Returning a degraded value instead would let the
  cache store a failure as truth for a whole TTL.
- **No silent unsigned wrapping at money boundaries.** `Long.toULong()` turns
  `-1` into 1.8e19; both platforms reject negative amounts instead.

## Status (this increment = walking skeleton)

Verified here (2026-07-29, this machine):

- ✅ `mobile/core` `cargo check` + `cargo build` on host.
- ✅ **`cargo build --target aarch64-apple-ios` succeeds** — the full Lexe SDK
  (in-process node) cross-compiles to iOS with default features. This resolves
  the brainstorm's biggest open risk: no thin-surface fallback is needed on iOS.
- ✅ UniFFI `generate` emits Kotlin + Swift bindings for all 12 core methods.

- ✅ All fixtures removed. `MockData.kt` / `MockWalletRepository` are gone; the
  Pool screen is driven by `OceanlnCore.pool_stats` (ocean.xyz) and the fiat
  toggle by `OceanlnCore.btc_usd` (mempool.space).
- ✅ Both source sets typecheck against the real generated bindings
  (`:composeApp:compileDebugKotlinAndroid`, `:composeApp:compileKotlinIosSimulatorArm64`).
- ✅ **iOS runs on the live Rust core** via the Swift↔Kotlin bridge. Verified on
  the iPhone 16 Pro simulator: real 24-word generation persisted at `0600`,
  Lexe node provisioned (`initWallet` + `createOffer`), real BOLT12 offer,
  BOLT11 invoice and BIP84 address in Receive, live ocean.xyz pool figures, and
  the wallet reloaded from disk across a relaunch.

Screens implemented: **Pool**, **Wallet** (simple + full), **Activity**,
**Node**, plus the **Send / Receive / Transaction-detail** sheets. Android wires
the live Rust core via the generated JNA bindings; iOS awaits the binding below.

## Open follow-ups

- **Collapse the iOS bridge boilerplate.** The Swift↔Kotlin bridge works but is
  hand-maintained: every new core method needs a Kotlin declaration, a Swift
  implementation and a mapper. [gobley](https://github.com/gobley/gobley)
  (KMP-native UniFFI bindings) would generate one Kotlin binding for both JVM
  and Native and delete `WalletCoreBridge` + `CoreBridge.swift` entirely.
- **iOS device builds** are wired and verified on an iPhone 14 Pro Max. Export
  your Team ID first — it is deliberately not committed:

  ```sh
  export OCEAN_DEVELOPMENT_TEAM=XXXXXXXXXX
  cd mobile/core && cargo build --release --target aarch64-apple-ios --lib
  cd ../iosApp && xcodegen generate
  xcodebuild -project iosApp.xcodeproj -scheme iosApp -configuration Debug \
    -sdk iphoneos -destination "platform=iOS,id=<device-udid>" \
    -derivedDataPath build-device -allowProvisioningUpdates build
  xcrun devicectl device install app --device <device-udid> \
    build-device/Build/Products/Debug-iphoneos/iosApp.app
  ```

  The core is linked by **explicit `.a` path per SDK**, never `-l<name>`: the
  crate emits both `staticlib` and `cdylib`, and with the target dir on the
  library search path the linker picks the `.dylib`. That still runs on the
  simulator, which shares the Mac's filesystem, so the app silently loads the
  core from the developer's machine — and dies on a real device with "Library
  not loaded".
- **Seed storage.** The core uses a file `SeedSource` under the app-data dir,
  created `0600`. On iOS that directory is marked `isExcludedFromBackup`, so the
  phrase does not sync to iCloud, and there is deliberately no temp-directory
  fallback (it is purgeable — a seed written there can vanish). Still to do:
  move to iOS Keychain / Android Keystore as a new `SeedSource` variant, which
  needs no call-site changes.
- **Per-worker list.** OCEAN's public API reports a live worker *count*
  (`active_worker_count`) but no per-worker rows, so the design's worker list is
  not rendered. It needs an upstream endpoint, not a client change.
- **Confirmation depth.** `Activity` carries a confirming block height but the
  node status exposes no chain tip, so the detail sheet shows the height rather
  than an "N / 6 confirmations" count.
- **Fonts.** Vendor Inter + Geist Mono into compose resources (see design/README).
- **CI.** Add a mobile workflow (macOS + Android SDK); the root `cargo --workspace`
  CI excludes `mobile/core` by design.
```
