#!/usr/bin/env bash
# Build KeyProxy.app from SwiftPM binary + daemon binary.
set -euo pipefail

REPO="$(cd "$(dirname "$0")/../.." && pwd)"
APP_SRC="$REPO/app"
DAEMON_SRC="$REPO/daemon"
OUT="$REPO/build"
BUNDLE="$OUT/KeyProxy.app"
CONFIG="${CONFIG:-release}"

echo "→ Building Swift executable ($CONFIG)..."
cd "$APP_SRC"
swift build -c "$CONFIG"
SWIFT_BIN="$APP_SRC/.build/$(swift build -c "$CONFIG" --show-bin-path)/KeyProxy"
# swift build --show-bin-path already prints the absolute path
SWIFT_BIN_PATH="$(swift build -c "$CONFIG" --show-bin-path)/KeyProxy"

echo "→ Building daemon ($CONFIG)..."
cd "$DAEMON_SRC"
if [ "$CONFIG" = "release" ]; then
  cargo build --release
  DAEMON_BIN="$DAEMON_SRC/target/release/keyproxyd"
else
  cargo build
  DAEMON_BIN="$DAEMON_SRC/target/debug/keyproxyd"
fi

echo "→ Assembling .app bundle at $BUNDLE"
rm -rf "$BUNDLE"
mkdir -p "$BUNDLE/Contents/MacOS" "$BUNDLE/Contents/Resources"
cp "$SWIFT_BIN_PATH" "$BUNDLE/Contents/MacOS/KeyProxy"
cp "$DAEMON_BIN" "$BUNDLE/Contents/Resources/keyproxyd"
cp "$APP_SRC/Resources/Info.plist" "$BUNDLE/Contents/Info.plist"

# Copy any additional SwiftPM-processed resources
if [ -d "$APP_SRC/.build/$CONFIG/KeyProxy_KeyProxy.bundle" ]; then
  cp -R "$APP_SRC/.build/$CONFIG/KeyProxy_KeyProxy.bundle" "$BUNDLE/Contents/Resources/"
fi

chmod +x "$BUNDLE/Contents/MacOS/KeyProxy" "$BUNDLE/Contents/Resources/keyproxyd"

echo "✓ Built $BUNDLE"
echo "  Run: open '$BUNDLE'"
