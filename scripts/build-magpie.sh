#!/usr/bin/env bash
# Build the Magpie gatherer as Respeek's Tauri sidecar,
# plus the hardware-name database it reads.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
TRIPLE="$(rustc -vV | sed -n 's/^host: //p')"
PROFILE="${1:-release}"

cd "$ROOT/vendor/magpie"
git submodule update --init --recursive >/dev/null
if [[ "$PROFILE" == "release" ]]; then cargo build --release; else cargo build; fi

install -Dm755 "target/$PROFILE/magpie" "$ROOT/src-tauri/binaries/respeek-magpie-$TRIPLE"

python3 platform-linux/hwdb/generate_hwdb.py --output "$ROOT/src-tauri/resources" \
  platform-linux/hwdb/20-pci-vendor-model.hwdb \
  platform-linux/hwdb/20-usb-vendor-model.hwdb \
  platform-linux/hwdb/30-linuxhw-wifi.hwdb
echo "Magpie sidecar: src-tauri/binaries/respeek-magpie-$TRIPLE"
ls -la "$ROOT/src-tauri/resources"
