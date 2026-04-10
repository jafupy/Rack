#!/bin/zsh

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
BUILD_DIR="$ROOT_DIR/.build"
RELEASE_DIR="$BUILD_DIR/release"
APP_DIR="$ROOT_DIR/dist/Rack.app"
MACOS_DIR="$APP_DIR/Contents/MacOS"
RESOURCES_DIR="$APP_DIR/Contents/Resources"
PLIST_PATH="$APP_DIR/Contents/Info.plist"
EXECUTABLE_PATH=""

mkdir -p "$BUILD_DIR" "$ROOT_DIR/dist"

swift build --configuration release --scratch-path "$BUILD_DIR"

if [[ -x "$BUILD_DIR/arm64-apple-macosx/release/Rack" ]]; then
  EXECUTABLE_PATH="$BUILD_DIR/arm64-apple-macosx/release/Rack"
elif [[ -x "$BUILD_DIR/x86_64-apple-macosx/release/Rack" ]]; then
  EXECUTABLE_PATH="$BUILD_DIR/x86_64-apple-macosx/release/Rack"
elif [[ -x "$RELEASE_DIR/Rack" ]]; then
  EXECUTABLE_PATH="$RELEASE_DIR/Rack"
else
  echo "Could not find release executable." >&2
  exit 1
fi

rm -rf "$APP_DIR"
mkdir -p "$MACOS_DIR" "$RESOURCES_DIR"

cp "$EXECUTABLE_PATH" "$MACOS_DIR/Rack"
chmod +x "$MACOS_DIR/Rack"

cat > "$PLIST_PATH" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "https://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleDevelopmentRegion</key>
  <string>en</string>
  <key>CFBundleExecutable</key>
  <string>Rack</string>
  <key>CFBundleIdentifier</key>
  <string>com.jafupy.Rack</string>
  <key>CFBundleInfoDictionaryVersion</key>
  <string>6.0</string>
  <key>CFBundleName</key>
  <string>Rack.</string>
  <key>CFBundleDisplayName</key>
  <string>Rack.</string>
  <key>CFBundlePackageType</key>
  <string>APPL</string>
  <key>CFBundleShortVersionString</key>
  <string>0.1.0</string>
  <key>CFBundleVersion</key>
  <string>1</string>
  <key>LSApplicationCategoryType</key>
  <string>public.app-category.developer-tools</string>
  <key>LSMinimumSystemVersion</key>
  <string>13.0</string>
  <key>LSUIElement</key>
  <true/>
  <key>NSHighResolutionCapable</key>
  <true/>
</dict>
</plist>
PLIST

echo "Built app bundle at $APP_DIR"
