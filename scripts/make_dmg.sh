#!/usr/bin/env bash
# Build the distributable DMG without mounting a temporary volume.
#
# Tauri's bundled create-dmg fork (bundle_dmg.sh) creates an interstitial
# RW image, mounts it, copies the app in, then unmounts. On machines with
# endpoint-security agents (e.g. CrowdStrike Falcon) the unmount of that
# temp volume fails with EBUSY (hdiutil error 49168, "Resource busy"),
# which breaks `tauri build --bundles dmg`.
#
# `hdiutil makehybrid` writes the HFS+ image directly from the folder
# without ever mounting it, so the EDR never gets a chance to hold it.
# The result mounts normally in Finder and contains the app.
set -euo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
REPO="$(dirname "$HERE")"
BUNDLE_DIR="$REPO/src-tauri/target/release/bundle"

VERSION="$(node -p "require('$REPO/package.json').version")"
# BUNDLE_DIR/ARCH are overridable for CI cross-target builds, whose bundle
# lands in target/<triple>/release and whose DMG name must carry the target
# arch rather than the runner's CPU.
BUNDLE_DIR="${BUNDLE_DIR:-$REPO/src-tauri/target/release/bundle}"
ARCH="${ARCH:-$(uname -m)}"
APP="$BUNDLE_DIR/macos/Runalytics.app"
DMG="$BUNDLE_DIR/dmg/Runalytics_${VERSION}_${ARCH}.dmg"

if [[ ! -d "$APP" ]]; then
  echo "error: $APP not found — run the app bundle first" >&2
  exit 1
fi

mkdir -p "$BUNDLE_DIR/dmg"
rm -f "$DMG"

# Staging folder: app + an Applications shortcut for drag-install.
STAGE="$(mktemp -d)"
trap 'rm -rf "$STAGE"' EXIT
cp -R "$APP" "$STAGE/"
ln -s /Applications "$STAGE/Applications"

hdiutil makehybrid -quiet -o "$DMG" -hfs -hfs-volume-name "Runalytics" "$STAGE"

echo "DMG created: $DMG"
