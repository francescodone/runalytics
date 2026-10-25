#!/usr/bin/env bash
# Build the distributable iOS IPA for a release.
#
#   ./scripts/build_ios.sh
#
# The IPA is deliberately built with `--no-sign`, so the app inside carries no
# signature at all. That is the whole point: Runalytics has no App Store
# distribution certificate, and a development-signed IPA only launches on the
# devices listed in its provisioning profile, so a signed build is useless to
# anybody but the machine that built it. An unsigned IPA is the raw bundle,
# which Sideloadly re-signs with each recipient's own Apple ID at install time
# — the iOS counterpart of the ad-hoc-signed DMG, which is likewise not
# notarised and needs one manual override on first launch.
#
# Env overrides for CI:
#   IPA_SRC   the IPA produced by the build, if you run `tauri ios build`
#             yourself instead of letting this script do it
#   OUT_DIR   where the versioned IPA is written
#   SKIP_BUILD=1  reuse an existing IPA rather than compiling
set -euo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
REPO="$(dirname "$HERE")"
APPLE="$REPO/src-tauri/gen/apple"

VERSION="$(node -p "require('$REPO/package.json').version")"
OUT_DIR="${OUT_DIR:-$APPLE/build}"

# --- 1. build ---------------------------------------------------------------
# `tauri ios build --no-sign` writes the archive and then zips the .app
# straight out of it, skipping the App Store export step entirely, so the
# result lands in build/<arch>/ rather than the signed build/ path.
if [[ "${SKIP_BUILD:-0}" != 1 && -z "${IPA_SRC:-}" ]]; then
  echo "==> building iOS archive (unsigned, release)"
  # The scaffold is committed under gen/apple, so no `tauri ios init` and no
  # CocoaPods: the Podfile declares zero pods, and the CLI only shells out to
  # `pod install` from `init`, never from `build`.
  (cd "$REPO" && npx tauri ios build --no-sign --target aarch64)
fi

# Newest match wins. `find | xargs ls -t` would list the current directory when
# find matches nothing, which fakes a success — so compare mtimes directly.
# The build/<arch>/ path is what --no-sign produces and the flat build/ path is
# what a signed build produces, so both are searched rather than assumed.
# Runalytics_*.ipa is excluded because that is this script's own versioned
# output: re-running with SKIP_BUILD=1 would otherwise select it and copy a
# file onto itself.
IPA_SRC="${IPA_SRC:-}"
if [[ -z "$IPA_SRC" ]]; then
  while IFS= read -r cand; do
    [[ -z "$IPA_SRC" || "$cand" -nt "$IPA_SRC" ]] && IPA_SRC="$cand"
  done < <(find "$APPLE/build" -name '*.ipa' -not -name 'Runalytics_*' -type f 2>/dev/null)
fi
[[ -n "$IPA_SRC" && -f "$IPA_SRC" ]] || {
  echo "error: no .ipa under $APPLE/build — check the build output above" >&2
  exit 1
}

# --- 2. gate: it must be unsigned --------------------------------------------
# The mirror of make_dmg.sh refusing to wrap an unsigned .app. Here the failure
# mode is inverted and silent: a development-signed IPA builds fine, uploads
# fine, and then refuses to open on every phone except the ones in the profile.
# Catching it here is cheaper than a recipient discovering it.
STAGE="$(mktemp -d)"
trap 'rm -rf "$STAGE"' EXIT
unzip -q "$IPA_SRC" -d "$STAGE"
APP="$(find "$STAGE/Payload" -maxdepth 1 -name '*.app' | head -1)"
[[ -n "$APP" ]] || { echo "error: $IPA_SRC has no .app under Payload" >&2; exit 1; }
if codesign -dv "$APP" >/dev/null 2>&1; then
  echo "error: $APP is code-signed — recipients could only install it on devices" >&2
  echo "       registered in that provisioning profile. Build with --no-sign so" >&2
  echo "       Sideloadly can sign it with the recipient's own Apple ID." >&2
  codesign -dv "$APP" 2>&1 | sed 's/^/       /' >&2
  exit 1
fi

# --- 3. publish-ready artifact -----------------------------------------------
mkdir -p "$OUT_DIR"
IPA="$OUT_DIR/Runalytics_${VERSION}.ipa"
cp "$IPA_SRC" "$IPA"

echo "IPA created: $IPA ($(du -h "$IPA" | cut -f1 | tr -d ' '))"
