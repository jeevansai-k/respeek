#!/usr/bin/env bash
# Install Respeek's desktop entry + icons for the current user so docks/app bars (COSMIC, GNOME,
# KDE) show the Respeek icon. Points at the release binary in this checkout.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
APP_ID="io.respeek.Respeek"
BIN="${1:-$ROOT/target/release/respeek}"
DATA="${XDG_DATA_HOME:-$HOME/.local/share}"
ICONS="$ROOT/src-tauri/icons"

install -Dm644 "$ROOT/branding/respeek.svg" "$DATA/icons/hicolor/scalable/apps/$APP_ID.svg"
install -Dm644 "$ICONS/32x32.png"        "$DATA/icons/hicolor/32x32/apps/$APP_ID.png"
install -Dm644 "$ICONS/64x64.png"        "$DATA/icons/hicolor/64x64/apps/$APP_ID.png"
install -Dm644 "$ICONS/128x128.png"      "$DATA/icons/hicolor/128x128/apps/$APP_ID.png"
install -Dm644 "$ICONS/128x128@2x.png"   "$DATA/icons/hicolor/256x256/apps/$APP_ID.png"
install -Dm644 "$ICONS/icon.png"         "$DATA/icons/hicolor/512x512/apps/$APP_ID.png"

mkdir -p "$DATA/applications"
cat > "$DATA/applications/$APP_ID.desktop" <<DESKTOP
[Desktop Entry]
Type=Application
Name=Respeek
Comment=A fluid system monitor
Exec=$BIN
Icon=$APP_ID
StartupWMClass=$APP_ID
Categories=System;Monitor;
Keywords=system;monitor;task;manager;cpu;memory;process;
Terminal=false
StartupNotify=true
DESKTOP

gtk-update-icon-cache -q -t "$DATA/icons/hicolor" 2>/dev/null || true
update-desktop-database -q "$DATA/applications" 2>/dev/null || true
echo "Installed $APP_ID.desktop → $BIN"
