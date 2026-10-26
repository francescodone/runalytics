#!/usr/bin/env bash
# Commit, tag, build and publish a Runalytics release.
#
#   ./scripts/release.sh v0.2.0
#
# Prerequisites:
#   - The version already bumped: ./scripts/bump_version.sh 0.2.0
#   - The `gh` CLI authenticated for the repository.
#
# What it does:
#   1. Commits the version bump (if any) and pushes it, then tags and pushes
#      the tag.
#   2. `npm run tauri build` to produce Runalytics.app, then
#      scripts/make_dmg.sh to produce the DMG with `hdiutil makehybrid`
#      (Tauri's own DMG step fails on machines whose endpoint-security agent
#      holds the interstitial mount busy).
#   3. scripts/build_ios.sh to produce the unsigned IPA that Sideloadly
#      installs. Set SKIP_IOS=1 to skip it (the iOS artifact then comes from
#      CI, which builds it too).
#   4. Creates the GitHub release with the .dmg, the .ipa, and the updater
#      artifacts (the signed .app.tar.gz, its .sig, and a generated
#      latest.json) — the .app.tar.gz + latest.json are what the in-app
#      updater in Settings → About downloads.
#
# The updater artifacts need the minisign key: tauri build fails outright
# when tauri.conf.json carries an updater pubkey but the environment has no
# TAURI_SIGNING_PRIVATE_KEY, so the key at ~/.tauri/runalytics.key is picked
# up automatically when present.
#
# Pass --dry-run to print what would happen without committing, tagging,
# building or publishing.
set -euo pipefail

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

DRY_RUN=0
if [[ "${1:-}" == "--dry-run" ]]; then
  DRY_RUN=1
  shift
fi

[[ $# -eq 1 ]] || { echo "usage: $(basename "$0") [--dry-run] <vX.Y.Z>" >&2; exit 1; }
TAG="${1#v}"
[[ "$TAG" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?$ ]] || {
  echo "error: '$TAG' is not a version" >&2; exit 1;
}
TAG="v$TAG"

CURRENT="$(node -p "require('./package.json').version")"
[[ "$TAG" == "v$CURRENT" ]] || {
  echo "error: tag $TAG does not match package.json ($CURRENT). Run scripts/bump_version.sh first." >&2
  exit 1
}

command -v gh >/dev/null || { echo "error: the gh CLI is required" >&2; exit 1; }

# Owner/repo comes from the git remote, so this works without gh auth too.
OWNER_REPO="$(git remote get-url origin | sed -E 's#^(https?://|git@)[^/]+[:/]##; s#\.git$##')"
[[ "$OWNER_REPO" == */* ]] || { echo "error: cannot parse owner/repo from the origin remote" >&2; exit 1; }

run() {
  if [[ "$DRY_RUN" == 1 ]]; then
    echo "  [dry-run] $*"
    return 0
  fi
  echo "  $*"
  "$@"
}

BRANCH="$(git rev-parse --abbrev-ref HEAD)"

# --- 1. commit + tag -------------------------------------------------------
echo "==> tagging $TAG"
if [[ -n "$(git status --porcelain)" ]]; then
  # Only the version bump should be outstanding at this point.
  echo "  committing working tree on $BRANCH"
  run git add -A
  run git commit -m "chore: release $TAG"
fi
run git tag "$TAG"
run git push origin "$BRANCH" "$TAG"

# --- 2. build --------------------------------------------------------------
ARCH="$(uname -m)"
BUNDLE_DIR="src-tauri/target/release/bundle"
DMG="$BUNDLE_DIR/dmg/Runalytics_${CURRENT}_${ARCH}.dmg"
IPA="src-tauri/gen/apple/build/Runalytics_${CURRENT}.ipa"
ASSETS=("$DMG")

# tauri build refuses to bundle when tauri.conf.json has an updater pubkey and
# no TAURI_SIGNING_PRIVATE_KEY is set, so pick the local key up automatically.
if [[ -z "${TAURI_SIGNING_PRIVATE_KEY:-}" && -f "$HOME/.tauri/runalytics.key" ]]; then
  export TAURI_SIGNING_PRIVATE_KEY="$HOME/.tauri/runalytics.key"
  export TAURI_SIGNING_PRIVATE_KEY_PASSWORD="${TAURI_SIGNING_PRIVATE_KEY_PASSWORD:-}"
fi

if [[ "$DRY_RUN" == 1 ]]; then
  echo "  [dry-run] npm run tauri build && bash scripts/make_dmg.sh"
  if [[ "${SKIP_IOS:-0}" == 1 ]]; then
    echo "  skipping the iOS build (SKIP_IOS=1) — CI publishes the IPA"
  else
    echo "  [dry-run] bash scripts/build_ios.sh"
    # Preview the asset list the publish step below would actually pass.
    ASSETS+=("$IPA")
  fi
else
  echo "==> building $TAG ($ARCH)"
  npm run tauri build
  bash scripts/make_dmg.sh
  [[ -f "$DMG" ]] || { echo "error: $DMG missing after build" >&2; exit 1; }

  # Needs Xcode, so it is the one step a machine without it can opt out of.
  # CI builds the same IPA, and the release stays valid without it locally.
  if [[ "${SKIP_IOS:-0}" == 1 ]]; then
    echo "  skipping the iOS build (SKIP_IOS=1) — CI publishes the IPA"
  else
    bash scripts/build_ios.sh
    [[ -f "$IPA" ]] || { echo "error: $IPA missing after build" >&2; exit 1; }
    ASSETS+=("$IPA")
  fi
fi

# --- 3. publish ------------------------------------------------------------
# The updater needs three things next to the DMG: the .app.tar.gz, the .sig
# tauri build already wrote beside it (when the signing key was in the
# environment), and a latest.json manifest pointing at both, keyed by
# platform. scripts/make_latest_json.mjs builds the manifest from the .sig.
TARBALL="$BUNDLE_DIR/macos/Runalytics.app.tar.gz"
if [[ "$DRY_RUN" == 1 ]]; then
  echo "  [dry-run] write latest.json, gh release create $TAG ${ASSETS[*]} $TARBALL …"
else
  if [[ -f "$TARBALL" && -f "$TARBALL.sig" ]]; then
    node scripts/make_latest_json.mjs "$BUNDLE_DIR/macos" "$OWNER_REPO" "$CURRENT"
    ASSETS+=("$TARBALL" "$TARBALL.sig" "$BUNDLE_DIR/macos/latest.json")
  else
    echo "  warning: no updater artifacts (missing $TARBALL or its .sig)" >&2
    echo "  warning: was TAURI_SIGNING_PRIVATE_KEY / ~/.tauri/runalytics.key available?" >&2
    echo "  warning: this release will not be offered to the in-app updater" >&2
  fi
  echo "==> creating release $TAG"
  gh release create "$TAG" "${ASSETS[@]}" --title "Runalytics $CURRENT" --generate-notes
fi

echo
echo "done: https://github.com/$OWNER_REPO/releases/tag/$TAG"
