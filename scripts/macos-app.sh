#!/usr/bin/env bash
# Wraps the compiled binary in the smallest .app bundle macOS accepts, so the
# game can be double-clicked instead of run from a terminal.
#   ./scripts/macos-app.sh target/dist/stardrop package/Stardrop.app
# Paths are relative to the repository root. Needs macOS: it calls codesign.
set -euo pipefail
cd "$(dirname "$0")/.."

binary="${1:?which binary to wrap}"
bundle="${2:?where to write the .app}"
version="$(sed -n 's/^version = "\(.*\)"$/\1/p' Cargo.toml | head -1)"

rm -rf "$bundle"
mkdir -p "$bundle/Contents/MacOS"
cp "$binary" "$bundle/Contents/MacOS/stardrop"
chmod +x "$bundle/Contents/MacOS/stardrop"

# No resources: the game carries its assets inside the executable (M5-7).
cat >"$bundle/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key>
    <string>Stardrop</string>
    <key>CFBundleDisplayName</key>
    <string>Stardrop</string>
    <key>CFBundleIdentifier</key>
    <string>dev.stardrop.game</string>
    <key>CFBundleExecutable</key>
    <string>stardrop</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleShortVersionString</key>
    <string>${version}</string>
    <key>CFBundleVersion</key>
    <string>${version}</string>
    <key>LSMinimumSystemVersion</key>
    <string>11.0</string>
    <key>NSHighResolutionCapable</key>
    <true/>
</dict>
</plist>
PLIST

# Ad-hoc signed, not with a developer identity. That is enough for the loader
# to run it on Apple Silicon, and not enough for Gatekeeper, which is why the
# README explains the one-time "right-click, Open". Notarising properly needs
# a paid Apple Developer account.
codesign --force --sign - "$bundle"
echo "==> $bundle ($version), ad-hoc signed"
