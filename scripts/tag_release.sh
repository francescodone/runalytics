#!/usr/bin/env bash
# Bump the version, commit it, tag it, and push — the whole CI trigger in one
# command. GitHub Actions (.github/workflows/release.yml) picks the tag up and
# builds + drafts the release.
#
#   ./scripts/tag_release.sh 0.2.0
#   ./scripts/tag_release.sh v0.2.0      # leading "v" is fine too
#   ./scripts/tag_release.sh 0.2.0 --dry-run
#
# For the local path instead (build the DMG here and publish with gh), use
# scripts/release.sh.
set -euo pipefail

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

DRY_RUN=0
ARGS=()
for arg in "$@"; do
  case "$arg" in
    --dry-run) DRY_RUN=1 ;;
    *) ARGS+=("$arg") ;;
  esac
done

[[ ${#ARGS[@]} -eq 1 ]] || {
  echo "usage: $(basename "$0") [--dry-run] <version>   e.g. 0.2.0" >&2
  exit 1
}
VERSION="${ARGS[0]#v}"
[[ "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?$ ]] || {
  echo "error: '$VERSION' is not a semver version (major.minor.patch[-prerelease])" >&2
  exit 1
}
TAG="v$VERSION"

command -v git >/dev/null || { echo "error: git is required" >&2; exit 1; }

BRANCH="$(git rev-parse --abbrev-ref HEAD)"
[[ "$BRANCH" != "HEAD" ]] || { echo "error: detached HEAD — check out a branch first" >&2; exit 1; }

run() {
  if [[ "$DRY_RUN" == 1 ]]; then
    echo "  [dry-run] $*"
    return 0
  fi
  echo "  $*"
  "$@"
}

# --- 1. bump ---------------------------------------------------------------
# bump_version.sh is a no-op when already at the requested version, so
# re-running this script after a partial failure is safe.
echo "==> bumping to $TAG"
if [[ "$DRY_RUN" == 1 ]]; then
  echo "  [dry-run] ./scripts/bump_version.sh $VERSION"
else
  ./scripts/bump_version.sh "$VERSION"
fi

# --- 2. commit the version files only ---------------------------------------
# Only the five declarations are staged, so unrelated work in progress is
# never swept into the release commit.
VERSION_FILES=(package.json package-lock.json src-tauri/tauri.conf.json \
               src-tauri/Cargo.toml src-tauri/Cargo.lock)

echo "==> committing $TAG on $BRANCH"
run git add -- "${VERSION_FILES[@]}"
if [[ "$DRY_RUN" == 1 ]]; then
  run git commit -m "chore: release $TAG"
elif git diff --cached --quiet; then
  echo "  nothing staged to commit (already at $TAG?)"
else
  run git commit -m "chore: release $TAG"
fi

if [[ "$DRY_RUN" == 0 ]] && [[ -n "$(git status --porcelain)" ]]; then
  echo "  note: other uncommitted changes were left out of the release commit:"
  git status --porcelain | sed 's/^/    /'
fi

# --- 3. tag + push ----------------------------------------------------------
if git rev-parse -q --verify "refs/tags/$TAG" >/dev/null; then
  echo "  tag $TAG already exists — reusing it"
else
  run git tag "$TAG"
fi

echo "==> pushing $BRANCH and $TAG"
run git push origin "$BRANCH" "$TAG"

echo
if [[ "$DRY_RUN" == 1 ]]; then
  echo "dry run complete — nothing was committed, tagged, or pushed."
else
  OWNER_REPO="$(git remote get-url origin | sed -E 's#^(https?://|git@)[^/]+[:/]##; s#\.git$##')"
  echo "done: CI is building $TAG now."
  echo "  watch:   gh run watch"
  echo "  release: https://github.com/$OWNER_REPO/releases (draft — publish when green)"
fi
