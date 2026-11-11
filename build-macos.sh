#!/usr/bin/env bash
# Compile ScreenCapture pour macOS et fabrique dist/ScreenCapture.app et dist/ScreenCapture-<version>-macOS.dmg.
#
#   ./build-macos.sh              binaire universel (Apple Silicon + Intel)
#   ./build-macos.sh --native     uniquement l'architecture de ce Mac (plus rapide)
#
# Signature : ad hoc par défaut ; définir MACOS_SIGN_IDENTITY="Developer ID Application: …" pour signer.
set -euo pipefail
cd "$(dirname "$0")"

VERSION=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n 1)
APP=dist/ScreenCapture.app
DMG=dist/ScreenCapture-$VERSION-macOS.dmg

if [[ "${1:-}" == "--native" ]]; then
    cargo build --release
    BIN=target/release/ScreenCapture
else
    rustup target add aarch64-apple-darwin x86_64-apple-darwin >/dev/null
    for t in aarch64-apple-darwin x86_64-apple-darwin; do
        cargo build --release --target "$t"
    done
    mkdir -p target/universal
    BIN=target/universal/ScreenCapture
    lipo -create -output "$BIN" \
        target/aarch64-apple-darwin/release/ScreenCapture \
        target/x86_64-apple-darwin/release/ScreenCapture
fi

[[ -f assets/ScreenCapture.icns ]] || cargo run --release --example make_icon

rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp "$BIN" "$APP/Contents/MacOS/ScreenCapture"
cp assets/ScreenCapture.icns "$APP/Contents/Resources/"
cp LICENSE THIRD-PARTY-LICENSES.txt "$APP/Contents/Resources/"
sed "s/@VERSION@/$VERSION/g" installer/macos/Info.plist > "$APP/Contents/Info.plist"

codesign --force --deep --sign "${MACOS_SIGN_IDENTITY:--}" "$APP"

STAGE=$(mktemp -d)
cp -R "$APP" "$STAGE/"
ln -s /Applications "$STAGE/Applications"
cp LICENSE "$STAGE/LICENSE.txt"
rm -f "$DMG"
hdiutil create -volname "ScreenCapture $VERSION" -srcfolder "$STAGE" -ov -format UDZO "$DMG" >/dev/null
rm -rf "$STAGE"

echo "Application : $APP"
echo "Image disque : $DMG"
