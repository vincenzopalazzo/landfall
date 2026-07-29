#!/usr/bin/env bash
# Build the OCEAN Lightning core for Android ABIs and emit the Kotlin bindings.
#
# Requires: Android NDK + `cargo-ndk` (`cargo install cargo-ndk`), ANDROID_HOME set.
#
# Output:
#   ../composeApp/src/androidMain/jniLibs/<abi>/liboceanln_mobile_core.so
#   ../composeApp/src/commonMain/kotlin/.../core/  (generated Kotlin)
set -euo pipefail
cd "$(dirname "$0")"

PROFILE="${1:-release}"
FLAG=""; [ "$PROFILE" = "release" ] && FLAG="--release"
# The UniFFI Kotlin bindings are JNA-based (JVM only), so they go in androidMain
# — commonMain must stay Kotlin/Native-clean for the iOS target.
JNI_OUT=../composeApp/src/androidMain/jniLibs
KT_OUT=../composeApp/src/androidMain/kotlin

if ! command -v cargo-ndk >/dev/null 2>&1; then
  echo "cargo-ndk not found. Install with: cargo install cargo-ndk" >&2
  exit 1
fi

for t in aarch64-linux-android armv7-linux-androideabi x86_64-linux-android; do
  rustup target add "$t" >/dev/null 2>&1 || true
done

echo "▸ building Android ABIs ($PROFILE)…"
cargo ndk -o "$JNI_OUT" \
  -t arm64-v8a -t armeabi-v7a -t x86_64 \
  build $FLAG --lib

echo "▸ generating Kotlin bindings…"
LIB="target/aarch64-linux-android/$PROFILE/liboceanln_mobile_core.so"
cargo run --quiet --bin uniffi-bindgen -- generate \
  --library "$LIB" --language kotlin --out-dir "$KT_OUT"

echo "✓ Android core ready:"
echo "  jniLibs: $JNI_OUT/<abi>/liboceanln_mobile_core.so"
echo "  Kotlin:  $KT_OUT/xyz/ocean/mobile/core/oceanln_mobile_core.kt (androidMain)"
