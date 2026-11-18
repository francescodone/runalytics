# Runalytics

Training analytics and adaptive running plans for macOS and iPhone.
Built with Tauri v2, React, TypeScript and Rust.

## Features

- **COROS sync** — activities, GPS routes, sleep, HRV, resting heart rate,
   fitness estimates and training load through the official
   [COROS MCP server](https://mcp.coros.com/mcp).
- **Adaptive plans** — 5K, 10K, half marathon and marathon training with
   structured workouts, recovery-aware adjustments and reviewed pace changes.
- **Training calendar** — week, month and plan views; move or skip sessions,
   match completed runs, export ICS and publish workouts to COROS.
- **Activity journal and maps** — searchable history, laps, splits and route heatmaps.
- **Health insights** — readiness, recovery, pace trends and a pain journal.
- **Optional integrations** — encrypted device plan sync, hosted calendar
   subscriptions and completed-workout descriptions in Strava.

## Install

Download a build from [GitHub Releases](https://github.com/francescodone/runalytics/releases).

- **macOS:** choose the Apple silicon or Intel DMG and drag Runalytics into
   Applications. Builds are ad-hoc signed, **not notarized**; first launch may
   require right-click → **Open**.
- **iPhone:** download `Runalytics_<version>.ipa` and install it through
   [Sideloadly](https://sideloadly.io), signing with your own Apple ID.
   Trust the developer in **Settings → General → VPN & Device Management**;
   enable **Developer Mode** under **Privacy & Security** on iOS 16 or later.
   Free Apple ID signatures expire after **seven days**: enable automatic
   refreshing in Sideloadly or re-sign before using the app again.

The iPhone build is distributed unsigned for sideloading, not through the App Store.

## Get started

1. Open Runalytics and connect your COROS account in Settings. Authorization
    opens your browser; credentials are stored in the device's Keychain.
2. Sync your activity and health history.
3. Create a plan in **Training**, choosing your goal, schedule and baseline.
4. Use **Activities** to review runs and **Health** to track recovery and
    review proposed adjustments.

During setup, leave threshold pace empty to use the watch's synced threshold
estimate, or estimate your roughly one-hour race pace from a recent race result.
This sets tempo and interval paces; if the watch has no estimate, enter one manually
or sync the watch before continuing. Leave weekly mileage empty to use recent
complete weeks of synced running history (a conservative baseline is used without
history). Your longest comfortable recent run sets the starting long-run distance;
leave it empty to estimate that distance from weekly mileage.

### Log out or switch COROS accounts

Each **COROS login ID identifies a separate user profile**. Plans, downloaded
history, settings, saved connections and unfinished drafts belong to that profile,
not to the watch model or the browser's current login session.

Open **Settings → Watch connection → Log out** and confirm. Logout waits for
active work, removes this device's COROS authorization and hides the profile
without deleting its data or ending plans on the watch. Select **Connect COROS**
to sign in again; authorization requests fresh authentication. The same COROS
account restores its saved profile, including after restarting the app. A different
COROS account starts with separate data and connection settings. Saved Strava,
GitHub and calendar credentials stay with their original profile.

On desktop, the browser returns to a temporary `127.0.0.1` callback served by
Runalytics. Keep the app open until sign-in finishes. If authorization is declined
or expires, retry **Connect COROS** in the app rather than reloading an old
callback tab. A native authentication update requires restarting the rebuilt app;
reloading the interface alone does not update its callback listener.

Local training remains usable offline while a profile is open. Logging out is
local, but signing back in requires COROS authorization. Existing account-specific
device-sync and conflict rules continue to apply within each profile.

Existing installations migrate automatically when their saved COROS login
identifies the owner. If previous local data has no verifiable owner, Runalytics
keeps it separate and offers **Import previous local data** after login. Confirm
only if that data belongs to the current COROS account. Import is available only
for an empty profile and never overwrites an existing user's plans or settings.

### Optional connections

- **Plan sync:** save a GitHub token with `gist` scope on both devices in
   Settings. Use the same COROS account and the same GitHub account's Gist channel.
   Plans are stored in an encrypted private Gist. Sync runs automatically once
   configured: setup asks for the connection, not a separate sync opt-in, and
   credentials can be added later in Settings. Update both apps before syncing.
- **Calendar subscription:** in **Settings → Calendar and devices**, provide
   an exact HTTPS upload-file URL and a direct, unauthenticated HTTPS read URL
   for the same ICS file, then enable automatic publishing. Use a dedicated file:
   each publication replaces its contents. Local ICS imports are snapshots,
   not subscriptions.
- **Strava:** create your own application in
   [Strava API settings](https://www.strava.com/settings/api), set the callback
   domain to `localhost`, and save its Client ID and Client Secret in
   **Settings → Strava**. Authorize activity read and write access on each device.
   Connecting enables description updates for newly recorded, plan-paired runs;
   existing notes are preserved and older activities are not backfilled.
- **Connection transfer:** use **Settings → Transfer connections** to export
   an encrypted QR with a passphrase of at least 12 characters. It can include
   calendar, GitHub and Strava application credentials, but not OAuth sessions
   or training history. Review imports, authorize providers separately, and
   keep the QR and passphrase private. Delete the image after use.

## Data and limitations

- Training history is stored locally in SQLite. On macOS, app data lives in
   `~/Library/Application Support/com.fradone.runalytics/`.
- Offline plan edits remain local until sync. If the remote Gist changed,
   **its snapshot replaces conflicting unpublished plan edits**; unrelated history
   and form drafts are retained. Simultaneous writes can still race.
- Calendar read URLs expose history, schedules and notes to anyone holding the
   link. Disabling publishing does not delete the hosted file or revoke its URL.
   Calendar apps control refresh timing. Strava descriptions inherit the
   activity's visibility and may be public. Disconnecting Strava stops local
   updates; revoke application access in Strava to disconnect it everywhere.
- Training scores and predictions are coaching heuristics, not medical advice
   or guaranteed race outcomes. Performance-based plan changes require approval.
- Garmin support is a feature-gated stub, not a working integration.
- Before production distribution, review the previously reported critical
   [MapLibre sanitizer advisory](https://github.com/advisories/GHSA-jrc7-96c5-q579)
   and complete native acceptance checks for live COROS, physical iPhone,
   updater and offline-map behavior. Browser checks do not verify these flows.

## Development

Requires Node.js/npm, Rust and Xcode with command-line tools for native Apple builds.
Google Chrome is required for the browser tests.

```sh
npm ci
npm run build
npm run test:ui
node --experimental-strip-types --test tests/profile-storage.unit.mjs
cargo test --manifest-path src-tauri/Cargo.toml
```

The production UI lives in [src/desk](src/desk/); native commands, storage and
training logic live in [src-tauri/src](src-tauri/src/).

For a browser-only preview, run the **Redesign preview** VS Code task and open
`http://localhost:1500/__desk_preview.html`. It uses synthetic IPC data, not
native integrations. Normal pages require the Tauri bridge for real data.
Native development uses `npm run tauri dev`; its startup hook clears port 1499,
so ensure no other process needs that port.

Build a macOS bundle with `npm run build:dmg`. For releases, use
[tag_release.sh](scripts/tag_release.sh) for CI or [release.sh](scripts/release.sh)
for a local build; both support `--dry-run`. The CI path commits, tags and pushes
version changes, then creates a draft release to publish after all jobs succeed.
Use only one release path per version.

## Documentation

- [Training methods and plan rules](docs/runna-like-plan-model.md)
- [Norwegian threshold model](docs/norwegian-threshold-model.md)
- [Codebase context](docs/codebase/ARCHITECTURE.md) — current, source-backed reference:
  [stack](docs/codebase/STACK.md), [structure](docs/codebase/STRUCTURE.md),
  [conventions](docs/codebase/CONVENTIONS.md), [integrations](docs/codebase/INTEGRATIONS.md),
  [testing](docs/codebase/TESTING.md) and [concerns](docs/codebase/CONCERNS.md).
- [Shared motion contracts](docs/design-system/motion.md)
- [Localization](docs/i18n.md)
- [Supabase foundation](docs/supabase-backend.md) — inactive; not a deployed backend.
- [Brand-agnostic users and COROS/Garmin parity plan](docs/ways-of-work/plan/wearable-integrations/garmin-parity/implementation-plan.md) — proposed shared-device user isolation and common activity, calendar, watch and Strava workflows; Garmin integration is not implemented.