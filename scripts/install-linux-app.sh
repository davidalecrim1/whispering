#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
APPIMAGE_PATH="${1:-}"
APPIMAGE_DIR="$ROOT_DIR/src-tauri/target/release/bundle/appimage"
INSTALL_DIR="${HOME}/.local/opt/Whispering"
INSTALL_PATH="${INSTALL_DIR}/Whispering.AppImage"
DESKTOP_DIR="${HOME}/.local/share/applications"
DESKTOP_FILE="${DESKTOP_DIR}/whispering.desktop"
ICON_DIR="${HOME}/.local/share/icons/hicolor/128x128/apps"
ICON_SOURCE="${ROOT_DIR}/src-tauri/icons/128x128.png"
ICON_PATH="${ICON_DIR}/whispering.png"

if [[ "$(uname -s)" != "Linux" ]]; then
  echo "install-linux-app.sh is only supported on Linux" >&2
  exit 1
fi

if [[ ! -d "$APPIMAGE_DIR" ]]; then
  echo "Linux AppImage directory not found: $APPIMAGE_DIR" >&2
  echo "Run 'make release-linux' first." >&2
  exit 1
fi

if [[ -z "$APPIMAGE_PATH" ]]; then
  APPIMAGE_PATH="$(find "$APPIMAGE_DIR" -maxdepth 1 -type f -name '*.AppImage' | sort | tail -n 1)"
fi

if [[ -z "$APPIMAGE_PATH" ]]; then
  echo "Linux AppImage not found under $APPIMAGE_DIR" >&2
  echo "Run 'make release-linux' first." >&2
  exit 1
fi

if [[ ! -f "$APPIMAGE_PATH" ]]; then
  echo "Linux AppImage not found: $APPIMAGE_PATH" >&2
  exit 1
fi

mkdir -p "$INSTALL_DIR" "$DESKTOP_DIR" "$ICON_DIR"
cp "$APPIMAGE_PATH" "$INSTALL_PATH"
chmod +x "$INSTALL_PATH"

if [[ -f "$ICON_SOURCE" ]]; then
  cp "$ICON_SOURCE" "$ICON_PATH"
fi

cat >"$DESKTOP_FILE" <<EOF
[Desktop Entry]
Type=Application
Name=Whispering
Comment=Local speech-to-text tray app
Exec=${INSTALL_PATH}
Icon=${ICON_PATH}
Terminal=false
Categories=Utility;AudioVideo;Audio;
StartupNotify=true
EOF

if command -v update-desktop-database >/dev/null 2>&1; then
  update-desktop-database "$DESKTOP_DIR" >/dev/null 2>&1 || true
fi

echo "Installed Linux AppImage: $INSTALL_PATH"
echo "Installed desktop entry: $DESKTOP_FILE"
echo "Launch Whispering from your desktop menu or run: $INSTALL_PATH"
