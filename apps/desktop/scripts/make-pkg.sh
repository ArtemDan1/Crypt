#!/bin/bash
# Builds the .pkg that in-app updates install (same approach as Verge).
# Run after `pnpm tauri build`. Output: target/release/bundle/pkg/Crypt_<version>_aarch64.pkg
set -euo pipefail
cd "$(dirname "$0")/.."

VERSION="$(node -p "require('./package.json').version")"
ROOT="$(cd ../.. && pwd)"
APP="$ROOT/target/release/bundle/macos/Crypt.app"
OUT="$ROOT/target/release/bundle/pkg"
BUNDLE_ID="$(node -p "require('./src-tauri/tauri.conf.json').identifier")"
[ -d "$APP" ] || { echo "Нет $APP — сначала pnpm tauri build" >&2; exit 1; }
BUILT="$(defaults read "$APP/Contents/Info.plist" CFBundleShortVersionString)"
[ "$BUILT" = "$VERSION" ] || { echo "Сборка $BUILT, а в package.json $VERSION — пересоберите" >&2; exit 1; }

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT
mkdir -p "$WORK/root/Applications" "$OUT"
cp -R "$APP" "$WORK/root/Applications/"
# No AppleDouble (._*) files in the payload.
xattr -rc "$WORK/root"

# Not relocatable: always install into /Applications, never over a dev build elsewhere.
pkgbuild --analyze --root "$WORK/root" "$WORK/component.plist"
plutil -replace 0.BundleIsRelocatable -bool NO "$WORK/component.plist"
pkgbuild --root "$WORK/root" --component-plist "$WORK/component.plist" \
  --identifier "$BUNDLE_ID" --version "$VERSION" --install-location / "$WORK/app.pkg"

cat > "$WORK/distribution.xml" <<XML
<?xml version="1.0" encoding="utf-8"?>
<installer-gui-script minSpecVersion="1">
  <title>Crypt $VERSION</title>
  <options customize="never" require-scripts="false" hostArchitectures="arm64"/>
  <!-- Installer asks to quit a running Crypt: replacing the bundle under it breaks it. -->
  <pkg-ref id="$BUNDLE_ID">
    <must-close><app id="$BUNDLE_ID"/></must-close>
  </pkg-ref>
  <choices-outline><line choice="default"><line choice="$BUNDLE_ID"/></line></choices-outline>
  <choice id="default"/>
  <choice id="$BUNDLE_ID" visible="false"><pkg-ref id="$BUNDLE_ID"/></choice>
  <pkg-ref id="$BUNDLE_ID" version="$VERSION">app.pkg</pkg-ref>
</installer-gui-script>
XML

PKG="$OUT/Crypt_${VERSION}_aarch64.pkg"
productbuild --distribution "$WORK/distribution.xml" --package-path "$WORK" "$PKG"
echo "$PKG"
