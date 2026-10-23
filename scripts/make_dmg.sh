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

# Gatekeeper refuses a quarantined bundle whose signature does not seal its
# resources ("code has no resources but signature indicates they must be
# present"), which is exactly what an unsigned bundler output looks like on
# another machine. Never publish a DMG around a bundle that fails this check.
# If this fails, bundle.macOS.signingIdentity in tauri.conf.json is missing.
VERIFY_LOG="$(mktemp)"
if ! codesign --verify --deep --strict "$APP" 2>"$VERIFY_LOG"; then
  echo "error: $APP is not properly code-signed:" >&2
  cat "$VERIFY_LOG" >&2
  exit 1
fi
rm -f "$VERIFY_LOG"

mkdir -p "$BUNDLE_DIR/dmg"
rm -f "$DMG"

# Staging folder: app + an Applications shortcut for drag-install.
STAGE="$(mktemp -d)"
trap 'rm -rf "$STAGE"' EXIT
cp -R "$APP" "$STAGE/"
# Drop any extended attributes (quarantine, provenance, resource forks) that
# may have crept into the built bundle, so the DMG ships only bundle content.
# makehybrid still stamps empty HFS+ FinderInfo on the volume, which is
# normal on-disk metadata and does not affect the signature or Gatekeeper.
xattr -cr "$STAGE/Runalytics.app"
ln -s /Applications "$STAGE/Applications"

hdiutil makehybrid -quiet -o "$DMG" -hfs -hfs-volume-name "Runalytics" "$STAGE"

echo "DMG created: $DMG"
