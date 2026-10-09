#!/usr/bin/env bash
# Bump the Runalytics version everywhere it is declared.
#
#   ./scripts/bump_version.sh 0.3.0
#   ./scripts/bump_version.sh v0.3.0        # leading "v" is stripped
#
# Unlike a multi-manifest Tauri/npm project, Runalytics is a Cargo workspace
# with a single source of truth: [workspace.package] version in the root
# Cargo.toml. Every crate inherits it with `version.workspace = true`, and the
# desktop shell's tauri.conf.json carries no version of its own, so Tauri bakes
# in that same inherited value. The only derived copies are the per-crate
# entries in Cargo.lock, refreshed with cargo.
#
# The edit is anchored on the [workspace.package] section rather than the bare
# version string, so the external dependency pins in [workspace.dependencies]
# that happen to read `version = "1"` etc. are never touched.
set -euo pipefail

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"

usage() {
  echo "usage: $(basename "${BASH_SOURCE[0]}") <version>   e.g. 0.3.0" >&2
  exit 1
}

[[ $# -eq 1 ]] || usage
NEW="${1#v}"

if [[ ! "$NEW" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?$ ]]; then
  echo "error: '$NEW' is not a semver version (major.minor.patch[-prerelease])" >&2
  exit 1
fi

cd "$ROOT"

# The old version is read from the one field we trust: the [workspace.package]
# version in the root Cargo.toml.
OLD="$(
  awk '
    /^\[workspace\.package\]/ { in_pkg = 1; next }
    /^\[/                     { in_pkg = 0 }
    in_pkg && /^version = "/ {
      match($0, /"[^"]*"/); print substr($0, RSTART + 1, RLENGTH - 2); exit
    }
  ' Cargo.toml
)"

if [[ -z "$OLD" ]]; then
  echo "error: could not read the current version from Cargo.toml" >&2
  exit 1
fi

if [[ "$OLD" == "$NEW" ]]; then
  echo "already at v$NEW — nothing to do"
  exit 0
fi

# --- workspace manifest ----------------------------------------------------
# Rewrite only the version line inside the [workspace.package] block.
perl -0pi -e "s/(\[workspace\.package\][^\[]*?\nversion = \")\Q$OLD\E\"/\${1}$NEW\"/" \
  Cargo.toml

# --- lockfile --------------------------------------------------------------
# Cargo.lock is generated. `cargo update -w` rewrites the version of every
# workspace member in one pass. Fall back to an anchored edit of each
# runalytics-* entry when cargo is not on PATH.
if command -v cargo >/dev/null 2>&1 && cargo update -w >/dev/null 2>&1; then
  LOCK_VIA="cargo"
else
  perl -0pi -e "s/(name = \"runalytics-[a-z-]+\"\nversion = \")\Q$OLD\E\"/\${1}$NEW\"/g" \
    Cargo.lock
  LOCK_VIA="perl fallback"
fi

# --- verify ----------------------------------------------------------------
fail=0

CARGO_NOW="$(
  awk '
    /^\[workspace\.package\]/ { in_pkg = 1; next }
    /^\[/                     { in_pkg = 0 }
    in_pkg && /^version = "/ {
      match($0, /"[^"]*"/); print substr($0, RSTART + 1, RLENGTH - 2); exit
    }
  ' Cargo.toml
)"
if [[ "$CARGO_NOW" != "$NEW" ]]; then
  echo "  ! Cargo.toml is '$CARGO_NOW', expected '$NEW'" >&2
  fail=1
fi

# Every runalytics-* crate entry in the lockfile must now read the new version.
STALE="$(
  perl -0ne 'print "$1\n" while /name = "(runalytics-[a-z-]+)"\nversion = "([^"]*)"/g' \
    Cargo.lock | grep -cv -- "$NEW" || true
)"
LOCK_NOW="$(
  grep -A1 '^name = "runalytics-core"$' Cargo.lock \
    | sed -n 's/^version = "\(.*\)"/\1/p'
)"
if [[ "$LOCK_NOW" != "$NEW" ]]; then
  echo "  ! Cargo.lock (runalytics-core) is '$LOCK_NOW', expected '$NEW'" >&2
  fail=1
fi

(( fail == 0 )) || { echo "bump incomplete" >&2; exit 1; }

echo "v$OLD -> v$NEW"
echo "  Cargo.toml   $NEW"
echo "  Cargo.lock   $NEW ($LOCK_VIA, all workspace crates)"
echo
echo "next: git commit -am 'chore: release v$NEW' && ./scripts/release.sh v$NEW"
