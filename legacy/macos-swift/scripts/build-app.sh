#!/usr/bin/env bash
# Builds "Aazad Voice.app" (and optionally a .dmg) into ./dist
#   ./scripts/build-app.sh            → release .app, ad-hoc signed
#   ./scripts/build-app.sh --dmg      → also create a drag-to-install DMG
#   SIGN_ID="Developer ID Application: …" ./scripts/build-app.sh --dmg   → distributable signed build
set -euo pipefail
cd "$(dirname "$0")/.."

VERSION="${VERSION:-0.1.0}"
BUILD="${BUILD:-$(date +%Y%m%d%H%M)}"
SIGN_ID="${SIGN_ID:--}"
APP="dist/Aazad Voice.app"

echo "▸ Compiling (release)…"
swift build -c release --arch arm64

echo "▸ Rendering icon…"
swift scripts/make-icon.swift . >/dev/null
mkdir -p build
iconutil -c icns build/AppIcon.iconset -o build/AppIcon.icns

echo "▸ Assembling bundle…"
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp .build/release/AazadVoice "$APP/Contents/MacOS/AazadVoice"
cp build/AppIcon.icns "$APP/Contents/Resources/AppIcon.icns"
sed -e "s/__VERSION__/$VERSION/" -e "s/__BUILD__/$BUILD/" Resources/Info.plist > "$APP/Contents/Info.plist"

echo "▸ Signing ($SIGN_ID)…"
if [[ "$SIGN_ID" == "-" ]]; then
  codesign --force --deep --sign - --entitlements Resources/AazadVoice.entitlements "$APP"
else
  codesign --force --deep --options runtime --timestamp --sign "$SIGN_ID" \
    --entitlements Resources/AazadVoice.entitlements "$APP"
fi
codesign --verify --deep --strict "$APP"

if [[ "${1:-}" == "--dmg" ]]; then
  echo "▸ Creating DMG…"
  DMG="dist/AazadVoice-$VERSION.dmg"
  STAGE="build/dmg"
  rm -rf "$STAGE" "$DMG"; mkdir -p "$STAGE"
  cp -R "$APP" "$STAGE/"
  ln -s /Applications "$STAGE/Applications"
  hdiutil create -volname "Aazad Voice" -srcfolder "$STAGE" -ov -format UDZO "$DMG" >/dev/null
  [[ "$SIGN_ID" != "-" ]] && codesign --sign "$SIGN_ID" --timestamp "$DMG"
  echo "✓ $DMG"
fi
echo "✓ $APP"
