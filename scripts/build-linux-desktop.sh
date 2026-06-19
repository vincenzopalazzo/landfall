#!/usr/bin/env bash
# Build the Tauri desktop app's Linux packages (.deb / .rpm / .AppImage)
# inside an Ubuntu container. Tauri's Linux bundlers link webkit2gtk/GTK and
# shell out to dpkg-deb / rpmbuild / appimagetool, none of which run on
# macOS — so on a Mac dev box this container is the way to produce them
# locally. (CI alternative: .github/workflows/desktop-linux.yml.)
#
# Requires Docker running. Output lands in ./dist-linux/ at the repo root.
# Linux build artifacts go to a container-internal target dir, so this does
# NOT clobber your macOS src-tauri/target.
#
# Usage: scripts/build-linux-desktop.sh
set -euo pipefail
cd "$(dirname "$0")/.."
REPO="$PWD"

if ! docker info >/dev/null 2>&1; then
  echo "Docker daemon not reachable — start Docker Desktop and retry." >&2
  exit 1
fi

# The web frontend is static (platform-independent), so build it on the host
# and let the container bundle the resulting dist/. ALWAYS rebuild: dist/ is
# gitignored and not arch-specific, so a stale dist/ left from an earlier run
# would silently package an outdated UI. A fresh build is cheap relative to the
# container's Rust compile.
echo "==> building web frontend (oceanln-web/dist)"
( cd oceanln-web && npm ci && npm run build )

mkdir -p dist-linux

echo "==> building Linux bundles in ubuntu:22.04 (first run is slow — full Rust + lexe SDK compile)"
docker run --rm \
  -v "$REPO":/work -w /work \
  -e CARGO_TERM_COLOR=always \
  ubuntu:22.04 bash -euo pipefail -c '
    export DEBIAN_FRONTEND=noninteractive
    apt-get update
    apt-get install -y \
      libwebkit2gtk-4.1-dev libgtk-3-dev librsvg2-dev \
      libayatana-appindicator3-dev libxdo-dev libssl-dev \
      patchelf file build-essential curl wget ca-certificates rpm
    curl --proto "=https" --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal
    . "$HOME/.cargo/env"
    cargo install tauri-cli --version "^2" --locked
    # Keep Linux artifacts out of the host mac target dir.
    export CARGO_TARGET_DIR=/tmp/target-linux
    cd src-tauri
    cargo tauri build --config "{\"build\":{\"beforeBuildCommand\":\"\"}}"
    mkdir -p /work/dist-linux
    cp -v /tmp/target-linux/release/bundle/deb/*.deb           /work/dist-linux/ 2>/dev/null || true
    cp -v /tmp/target-linux/release/bundle/rpm/*.rpm           /work/dist-linux/ 2>/dev/null || true
    cp -v /tmp/target-linux/release/bundle/appimage/*.AppImage /work/dist-linux/ 2>/dev/null || true
  '

echo "==> Linux packages in dist-linux/:"
ls -la dist-linux/
