#!/bin/sh
# Builds Flick.app next to this script.
#
#   ./build.sh                          ad-hoc signed (fine for trying it out)
#   SIGN_IDENTITY="Apple Development: …" ./build.sh
#                                       signed with your certificate, so the
#                                       Accessibility permission survives rebuilds
set -eu
cd "$(dirname "$0")"
cargo build --release

VERSION=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
APP=Flick.app
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp target/release/flick "$APP/Contents/MacOS/flick"
cp assets/Flick.icns "$APP/Contents/Resources/Flick.icns"
cat > "$APP/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
  <key>CFBundleExecutable</key><string>flick</string>
  <key>CFBundleIdentifier</key><string>io.github.georgenijo.flick</string>
  <key>CFBundleName</key><string>Flick</string>
  <key>CFBundleDisplayName</key><string>Flick</string>
  <key>CFBundleIconFile</key><string>Flick</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>$VERSION</string>
  <key>CFBundleVersion</key><string>$VERSION</string>
  <key>LSMinimumSystemVersion</key><string>14.0</string>
  <key>LSUIElement</key><true/>
  <key>NSHighResolutionCapable</key><true/>
  <key>NSHumanReadableCopyright</key><string>MIT License</string>
</dict></plist>
PLIST
codesign --force --sign "${SIGN_IDENTITY:--}" "$APP"
echo "built $(pwd)/$APP"
