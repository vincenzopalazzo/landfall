#!/usr/bin/env bash
# Build the OCEAN Lightning core for iOS and emit the Swift bindings.
#
# Output:
#   generated/swift/                 — OceanlnMobileCore.swift + FFI header/modulemap
#   generated/OceanlnMobileCore.xcframework — device + simulator static libs
#
# The full Lexe SDK (in-process node) cross-compiles to aarch64-apple-ios with
# default features — verified 2026-07-29 — so no feature juggling is needed here.
set -euo pipefail
cd "$(dirname "$0")"

PROFILE="${1:-release}"
FLAG=""; [ "$PROFILE" = "release" ] && FLAG="--release"
DEVICE=aarch64-apple-ios
SIM=aarch64-apple-ios-sim
LIBNAME=liboceanln_mobile_core.a
OUT=generated

rustup target add "$DEVICE" "$SIM" >/dev/null 2>&1 || true

echo "▸ building $DEVICE ($PROFILE)…"
cargo build $FLAG --target "$DEVICE" --lib
echo "▸ building $SIM ($PROFILE)…"
cargo build $FLAG --target "$SIM" --lib

echo "▸ generating Swift bindings…"
rm -rf "$OUT/swift" && mkdir -p "$OUT/swift"
cargo run --quiet --bin uniffi-bindgen -- generate \
  --library "target/$DEVICE/$PROFILE/liboceanln_mobile_core.dylib" \
  --language swift --out-dir "$OUT/swift" 2>/dev/null || \
cargo run --quiet --bin uniffi-bindgen -- generate \
  --library "target/$DEVICE/$PROFILE/$LIBNAME" \
  --language swift --out-dir "$OUT/swift"

# UniFFI emits <module>FFI.modulemap; xcframework wants it named module.modulemap.
cp "$OUT/swift/OceanlnMobileCoreFFI.modulemap" "$OUT/swift/module.modulemap"

echo "▸ assembling xcframework…"
rm -rf "$OUT/OceanlnMobileCore.xcframework"
xcodebuild -create-xcframework \
  -library "target/$DEVICE/$PROFILE/$LIBNAME"  -headers "$OUT/swift" \
  -library "target/$SIM/$PROFILE/$LIBNAME"     -headers "$OUT/swift" \
  -output "$OUT/OceanlnMobileCore.xcframework"

echo "✓ iOS core ready: $OUT/OceanlnMobileCore.xcframework"
echo "  Swift glue:    $OUT/swift/OceanlnMobileCore.swift"
