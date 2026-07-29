package xyz.ocean.mobile.data

// iOS core binding.
//
// The Rust core cross-compiles cleanly to aarch64-apple-ios (verified
// 2026-07-29), and UniFFI emits Swift bindings via ../core/build-ios.sh. What's
// left is bridging that Swift API into Kotlin/Native — either through a small
// Swift↔Kotlin shim, or (preferred) by generating KMP-native UniFFI bindings
// with gobley so one Kotlin binding serves both JVM (Android) and Native (iOS).
// Tracked in mobile/README.md → "iOS core binding". Until then iOS renders the
// design's mock data.
actual fun createCoreRepository(appDataDir: String): WalletRepository? = null
