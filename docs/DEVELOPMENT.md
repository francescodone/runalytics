# Runalytics — Development

For what the app does and how it's structured, see the [README](../README.md).
For the agent contract, see [`SKILL.md`](../SKILL.md).

## TL;DR — cut a release

```sh
./scripts/bump_version.sh 0.3.0            # version lives only in Cargo.toml
git commit -am "chore: release v0.3.0"
./scripts/release.sh v0.3.0                # tags + pushes → CI builds the bundles
```

Pushing the `v0.3.0` tag triggers [`.github/workflows/release.yml`](../.github/workflows/release.yml),
which runs the `aarch64`/`x86_64` matrix via `tauri-action` and opens a **draft**
GitHub release. Publish it from the [Releases page](https://github.com/francescodone/runalytics/releases)
once both jobs are green.

- **One source of truth.** The version is `[workspace.package].version` in the
  root [`Cargo.toml`](../Cargo.toml). Every crate inherits it with
  `version.workspace = true`, and `tauri.conf.json` carries no version of its
  own, so the bundle gets that same value. `bump_version.sh` also refreshes the
  per-crate entries in `Cargo.lock` (`cargo update -w`).
- **Signing.** Release builds sign the updater artifacts with the minisign key
  stored as the repository secret `TAURI_SIGNING_PRIVATE_KEY`. It is not Apple
  Developer ID signing, so first launch still needs the *Open Anyway* flow.
- **CI, not local.** `release.sh` only tags and pushes — the two-target build
  and signing happen in CI. It refuses to run with a dirty tree, a tag that
  already exists, or a tag that doesn't match `Cargo.toml`.

## Scripts

| Script | Purpose |
| --- | --- |
| [`scripts/bump_version.sh`](../scripts/bump_version.sh) `<X.Y.Z>` | Bumps `[workspace.package].version` and refreshes every `runalytics-*` entry in `Cargo.lock`. Idempotent and semver-validated. |
| [`scripts/release.sh`](../scripts/release.sh) `<vX.Y.Z>` | Verifies the tag matches `Cargo.toml`, then creates and pushes the annotated tag that triggers the release workflow. |
