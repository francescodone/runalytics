#!/usr/bin/env bash
# Cut a Runalytics release: tag the current commit and push it so CI builds
# and publishes the macOS bundles.
#
#   ./scripts/release.sh v0.3.0
#
# Prerequisites:
#   - The version already bumped and committed: ./scripts/bump_version.sh 0.3.0
#   - A clean working tree on a commit you are happy to ship.
#   - The repository secret TAURI_SIGNING_PRIVATE_KEY configured so the
#     updater artifacts can be signed (see .github/workflows/release.yml).
#
# What it does:
#   1. Verifies the tag matches the version in Cargo.toml.
#   2. Refuses to run with a dirty tree or an already-existing tag.
#   3. Creates an annotated tag and pushes it. The push triggers
#      .github/workflows/release.yml, which runs the aarch64/x86_64 matrix via
#      tauri-action and opens a DRAFT GitHub release — publish it from the
#      Releases page once both jobs are green.
#
# Unlike clipstack's release.sh, this does not build locally: Runalytics has no
# updater private key on developer machines, and the two-target matrix build is
# CI's job.
set -euo pipefail

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

[[ $# -eq 1 ]] || { echo "usage: $(basename "$0") <vX.Y.Z>" >&2; exit 1; }
VER="${1#v}"
[[ "$VER" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?$ ]] || {
  echo "error: '$VER' is not a version" >&2; exit 1;
}
TAG="v$VER"

CURRENT="$(
  awk '
    /^\[workspace\.package\]/ { in_pkg = 1; next }
    /^\[/                     { in_pkg = 0 }
    in_pkg && /^version = "/ {
      match($0, /"[^"]*"/); print substr($0, RSTART + 1, RLENGTH - 2); exit
    }
  ' Cargo.toml
)"
[[ "$VER" == "$CURRENT" ]] || {
  echo "error: tag $TAG does not match Cargo.toml ($CURRENT). Run scripts/bump_version.sh first." >&2
  exit 1
}

command -v git >/dev/null || { echo "error: git is required" >&2; exit 1; }

if [[ -n "$(git status --porcelain)" ]]; then
  echo "error: working tree is dirty — commit the version bump first" >&2
  exit 1
fi

if git rev-parse -q --verify "refs/tags/$TAG" >/dev/null; then
  echo "error: tag $TAG already exists" >&2
  exit 1
fi

echo "==> tagging $TAG at $(git rev-parse --short HEAD)"
git tag -a "$TAG" -m "Runalytics $TAG"

echo "==> pushing $TAG"
git push origin "$TAG"

echo
echo "done: CI is building the release at"
echo "  https://github.com/francescodone/runalytics/actions"
echo "publish the draft once both matrix jobs are green:"
echo "  https://github.com/francescodone/runalytics/releases"
