# Runalytics

Training analytics for runners. A Tauri v2 app (macOS and iPhone) that pulls
your COROS data, scores your training, builds adaptive race-preparation plans,
and shows where you run.

The Mac is where a plan is created and published; the phone carries it, so the
two stay in step over the plan-sync channel in Settings. See
[Installing on iPhone](#installing-on-iphone) for how the iOS build is
distributed.

## Features

- **COROS integration** over the official [COROS MCP server]
  (`https://mcp.coros.com/mcp`, OAuth 2.0 + PKCE). Connect opens your browser;
  tokens live in the macOS keychain and auto-refresh.
- **Sync** activities (with FIT tracks), daily health (sleep, HRV, resting HR,
  stress), fitness assessment (VO₂max, threshold, predicted times) and
  training load.
- **Training**: one workspace for the current week, month and entire plan.
  Desktop uses content-sized session tiles and a modal session inspector; mobile uses a
  date ribbon, daily itinerary and full-screen detail. Connection and initial
  sync actions appear when needed.
- **Journal**: searchable activity history with sport, source and date filters;
  selected-run routes, recorded laps, kilometre splits, prescribed targets and
  stored training load. A separate training map shows all recorded routes.
- **Health**: readiness, recovery, training load, fitness, wearable
  history, pain episodes, reviewed adjustments and consistency milestones.
- **Pace insights**: Health > Pace insights compares the last five eligible
  Speed (interval/tempo) or pace-targeted Long sessions. The chart shows each
  session's fastest and slowest target-relative work reps, with an inspectable
  rep table, outdated-session indicators, activity links and the next workout.
  Pace changes still use the existing review-and-approve workflow.
- **Navigation**: Training, Activities and Health are the three primary workspaces.
  Desktop has a floating navigation rail; mobile has three bottom destinations.
  Settings and Help remain utility actions. Settings uses a searchable directory
  and focused task editors, not a second sidebar. The independent React
  presentation uses locally bundled Outfit, neutral light and graphite dark
  surfaces, a coral default accent and six accent palettes. Rounded controls,
  evenly spaced day lanes and a centered floating mobile navigation share the
  same geometry and spacing tokens. The mobile workspace extends to the viewport edges; its
  workspace and height-bounded activity and plan lists own scrolling. Navigation
  and tabs use the same hover, pressed and selected treatments as buttons.
  Information buttons open dialogs with a blurred backdrop, contained keyboard
  focus and nested scroll locks. Desktop content reveals once on viewport entry,
  settling from a brief blur and small staggered offsets. Desktop outgoing views
  scatter before removal; focus or a press immediately clears an incoming reveal.
  Compact view changes update in place with a brief nonblocking fade, without
  waiting for exits or staggered reveals. Visited mobile workspaces retain local
  state while hidden effects are suspended; returning revalidates their data.
  Week, Month and Plan reuse the loaded calendar range instead of fetching again
  for each horizon switch.
  Charts have no persistent blur layers. Reduced motion removes blur and movement;
  reduced transparency, increased contrast and forced colors have explicit
  fallbacks. The font is OFL-licensed. Shared visual and motion contracts live in
  `src/desk/tokens.css`, `src/desk/interaction.tsx` and `src/desk/interaction.css`.
- **Plans**: 5K / 10K / half / marathon plans to a target date from your
  threshold pace, weekly frequency, long-run day, and mileage-increase method
  (+5 / +8 / +12 % per week). Geometric ramp with 3-build/1-down cycles,
  taper, and race week.
  Creation uses Direction, Baseline and Review steps, including a start date
  that defaults to today. Choose one or more available tempo/interval days at
  least two days from the long run. The plan uses only those days for quality
  workouts, subject to spacing, recovery and training-load rules; some weeks
  have fewer quality sessions. Workouts begin no earlier than the selected day;
  maintenance block lengths are counted from that date. The plan horizon combines
  a week-by-week roadmap with inputs, predictions, publishing and management.
  Each visit to Plan centers the current week. Creation is unavailable while a
  plan is active. **End current plan** removes only the current plan and its
  sessions; **Clear plans and scheduled training** in Settings also removes older plans,
  drafts and pending plan offers. Both require confirmation and preserve
  recorded activities, health history and pain entries.
  Clearing is not local-only: it attempts to publish the cleared plan to the
  existing GitHub Gist when device sync is enabled; other devices receive it
  on their next successful sync. It also attempts COROS schedule cancellation
  and refreshes the local calendar file and any enabled remote calendar feed.
  The Gist and COROS plan entries themselves are not deleted. Calendar apps
  fetch updates on their own schedule; imported file snapshots are unchanged.
  Remote cleanup can fail after local deletion, so check sync results afterward.
- **Adaptive adjustments**: after every sync, poor recovery (sleep / RHR / HRV
  / ACWR) or a struggled session softens the next hard workout — never the
  other way around.
- **Manual difficulty**: making a plan harder can transfer distance from an
  unfinished easy run to a quality session in the same week, keeping weekly
  mileage unchanged. Easy runs retain at least 3 km. When a transfer is not
  available, the adjustment can instead change the work within the session or
  its pace. Recovery, load and protected-week limits still apply; Undo restores
  the affected distances and prescriptions.
- **Performance adaptations**: sync and manual activity matching analyze measured
  tempo/interval reps. Consistent evidence can propose bounded changes to quality
  paces and upcoming quality distances, with before/after race estimates. Nothing
  changes until you approve; watch fitness predictions remain separate.
- **Races and tests**: Training > Schedule race or test adds a 5 km parkrun,
  tune-up race or 1-10 km time trial at least eight days before the plan target.
  It replaces that day's workout and eases adjacent hard or long sessions.
  After syncing, match the race-only activity to the benchmark. A recorded
  distance within 5% of the scheduled distance can support a 5 s/km quality-pace
  adjustment using the plan's race model. Changes require approval; faster
  targets also require healthy recovery and load. Completed runs, the target
  race and watch fitness measurements are preserved.
- **Calendar**: inspect workouts in Training, including warm-up,
  repetitions, recovery, cool-down, and pace ranges. Completed runs replace
  matched planned events and retain the original plan in their notes.
  Week, Month and Plan share the same scheduling workspace. Move sessions with
  Undo, skip/restore them, preview a week reset and match recorded activities.
  An ICS feed can be published automatically to your HTTPS/WebDAV host.
  Push workouts to COROS's training schedule from Training.
- **Connection transfer**: Settings > Transfer connections offers
  **Create transfer QR** and **Import transfer QR**. Choose a private passphrase
  of at least 12 characters on the old device, then scan or upload the encrypted
  QR image on the new device and enter the same passphrase. Initial setup offers
  the import first. Sign in to the same COROS account to verify the transfer;
  after confirmation, setup attempts to sync COROS history and the plan when a
  device-sync token is included. Failures can be retried. Review before
  replacing settings. Calendar hosting credentials, the GitHub sync token and
  saved Strava application Client ID/Secret are included; COROS/Strava sessions
  and training history are not. The review shows actual URLs, username and
  Strava Client ID, with secrets masked and replacement/removal effects stated.
  Older QR images without Strava credentials leave Strava unchanged. Imports leave
  calendar publishing paused; a transferred GitHub token enables device sync.
  Strava still requires separate authorization. Camera scanning is available on iPhone and supported browsers;
  native macOS uses image upload. Keep both the image and passphrase private and
  delete the image after use. Transfer images remain reusable until deleted or
  the included credentials are revoked.
- **Pain journal**: a dedicated Health view supports region selection, intensity,
  onset, notes, follow-up, resolve/reopen and confirmed deletion. Proposed plan
  changes require review. Optional assistant conversations require consent and
  can be cleared separately from the pain record.
- **Watch connection**: Settings shows the next setup step or a direct Sync now
  action for connected accounts. Connection diagnostics open in a dialog.
- **Strava descriptions**: optional, client-only connection using your own
  Strava API credentials. New plan-paired runs receive the completed session's
  copied description during watch sync, with durable local tracking and checks
  for copies already present from another device. Existing notes stay intact.
- **Heatmap**: MapLibre GL JS with a heatmap layer over OSM raster tiles, or
  fully offline from a [PMTiles] archive (path configurable in Settings).
- **Plan sync**: desktop and phone share an encrypted private GitHub Gist,
  keyed to the same COROS account. Enable plan sync and save a GitHub token
  with gist scope on both devices. Both devices can edit, but the Gist wins
  when unpublished local plan changes conflict with a changed remote copy.
  Foreground, reconnect and visible-app checks keep the local plan current;
  session completions and plan endings travel between devices too.
- **Garmin**: feature-gated (`--features garmin`); no official public API
  exists, so it ships as a stub.

## Strava Setup

1. Create a personal API application at
  [Strava API settings](https://www.strava.com/settings/api). Each user supplies
  their own Client ID and Client Secret; Runalytics has no OAuth server and
  does not bundle shared API credentials. Follow Strava's current API terms and
  application-capacity requirements.
2. Set the application's **Authorization Callback Domain** to `localhost`.
  On iPhone, the app uses the callback
  `com.fradone.runalytics://localhost/strava/callback` with Apple's native browser
  authentication session. macOS opens the system browser and uses a temporary
  `http://127.0.0.1:<port>/strava/callback`; Strava whitelists the loopback host.
3. In **Settings > Strava**, save the Client ID and Client Secret, then select
  **Connect with Strava**. Allow both activity read access, including private
  activities, and activity write access. Configure and authorize each device
  separately. The secret and rotating tokens remain in that device's Keychain;
  they are never sent through plan sync, calendar publishing or diagnostics.
4. Enable **Update paired activity descriptions** and confirm the privacy
  notice. The payload is the same completed-session text as **Copy
  prescription**, including results and training-load information.
  It inherits the activity's Strava visibility and may therefore be public.

Only activities recorded from the first activation time on that device/account
are eligible. Downloading an older activity or pairing it later does not bypass
this cutoff. Manual pairing is picked up by the next watch sync. Pause/resume
and reconnect retain the original cutoff and completion receipts.

Watch sync, including launch sync, queues eligible pairs and attempts a bounded
batch. Delayed watch uploads to Strava remain pending, including on syncs with
zero new watch activities. **Retry pending descriptions** respects persisted
cooldowns; it is not historical backfill. Nothing runs while the app is closed.
Ambiguous or inconsistent matches are not updated automatically. Reconnect when
Strava reports missing permissions or expired authorization; automation remains
paused after reconnection until explicitly enabled again.

Existing descriptions are preserved and a single Runalytics block is appended
with a stable sync reference. A matching reference or exact clipboard copy is
adopted without rewriting. Later edits to a confirmed description are not
automatically restored. Two devices running simultaneously, or other apps
editing the same description, can still race: this is not a distributed lock or
an exactly-once HTTP guarantee.

Disconnecting stops local updates and removes local OAuth tokens, without
revoking another device or modifying any Strava activity. **Remove API
credentials** also removes the locally saved Client ID and Client Secret.
Local completion receipts remain to prevent accidental recopying. To revoke
the API application's access everywhere, use Strava's connected-app settings.
Strava may invalidate another device's token chain when the same personal API
application is reauthorized; reconnect that device if needed, rather than
copying tokens between devices.

Queued copied text is discarded after a confirmed update or adoption. Pending
text expires after 90 days and is cleared during subsequent sync housekeeping;
expired items require attention rather than being rewritten automatically.
Compact receipts follow the lifetime of the local activity history and are
removed on sync when their source activity no longer exists. Original calendar
notes remain part of the existing local training history. Remote descriptions
are read transiently and are not stored as an additional Strava activity feed.

## UI Verification

### Pace Insights Calculation

Pace insights are inspired by [Runna's chart](https://support.runna.com/en/articles/10751290-pace-insights-chart),
not a reproduction of its unpublished status algorithm. Each rep's position is
`(target midpoint - measured seconds/km) / max(target half-width, 1 second/km)`.
The band spans -1 to +1; higher means faster relative to that rep's own target.
The session count means every included rep was within its prescribed range;
it is separate from the adjustment engine's tolerance and consecutive-session rule.

Only linked, completed sessions in the current plan with usable GPS data count.
Work-lap boundaries follow the planned distances without stretching a short
track to fill the prescription. Reps need at least 98% distance coverage and a
finite pace between 2:00 and 20:00/km. Warm-ups, recoveries, missing/incomplete
reps, invalid coordinates and non-increasing timestamps are excluded. This
does not identify every GPS outlier or an omitted rep in the middle of a run;
these are inferred planned segments, not imported watch laps.

Targets are reconstructed from the current plan, not immutable historical
snapshots. Speed workouts before the latest accepted pace adjustment remain
visible as outdated but do not enter the current count. Conversational long
runs have no targets and are not graded; long-run insights never trigger
quality-pace adjustments. No pace change is applied by viewing the chart.

### Reviewed Training Adaptation

The adjustment engine uses the same measured work reps, not whole-run average
pace. At least 80% of prescribed work reps must be measurable; at least 75% of
measured reps must be faster or slower than their own range plus the configured
tolerance (4 seconds/km by default). The default requirement is three consecutive
eligible sessions in the same direction; the configurable two-to-six-session
rule is independent of the chart's five-session display. Mixed or on-target
sessions break the streak. Sessions under superseded targets do not count again.

Each offer moves future quality pace by 5 seconds/km, capped at a total offset
of +/-20 seconds/km. Only planned tempo/interval distances in the next 14 days
can change: approximately +3% for progression or -5% for easing, rounded to 10m.
Completed, skipped, easy, long and race sessions keep their distances, as do
sessions in the final 14 days before the target date. Faster progression requires
normal-or-better recovery, no unresolved pain and a finite
load ratio no higher than 1.3. Missing load evidence prevents progression.

Review shows the proposed distance and shifted pace range before approval.
Acceptance rechecks current evidence, safety and the proposed workout changes;
offset, distance edits and decision status are committed atomically. Changed
workouts reject stale offers, and replay cannot apply an offer twice. External
calendar/watch publishing follows the local commit and can still fail separately.

Training race estimates use the plan threshold plus its accepted quality offset
as a one-hour pace and project 5K, 10K, half and marathon times with the Riegel
exponent 1.06. These are coaching heuristics, not measured fitness or a copy of
Runna's proprietary algorithm. Course, endurance, weather and fatigue are not
modeled. Watch/provider predictions are never overwritten.

### Health Percentages

Readiness, recovery and training scores use fixed 0-100% axes, not
the minimum and maximum of the selected period. Readiness history uses the same
training/recovery formula as the current score: `(45 * training + 35 * recovery) / 80`,
or 56.25% training and 43.75% recovery. Reported pain remains a separate safety
input for reviewed plan adjustments, not a component of readiness.
Normalized training load is `100 * raw ratio / configured optimized-load ceiling`,
clamped to 0-100%. Values above the ceiling remain 100%; the unmodified ratio and
short-/long-term load remain visible. Missing values are not rendered as zero.

### Browser Checks

`npm run test:ui` runs the Playwright regression suites using synthetic
harness data and an installed Google Chrome. It starts Vite on port 1500 if no
preview is already running. `npm run build` checks TypeScript and production
page entries, including Support. Screenshots and failure traces go into the
ignored `test-results/` directory.

The **Verify UI system unification** VS Code task runs the production build,
the complete browser suite without exclusions, and `git diff --check`. It writes
`test-results/ui-system-report.json` and failure artifacts under
`test-results/ui-system`. Coverage includes 320/390/768/1440px light/dark layouts,
all settings editors and collapsed diagnostics, all six palettes' interaction
states, Axe checks, keyboard focus, touch dismissal, accessibility preferences,
map pixels/zoom, failed drafts, Undo, reviewed mutations, credential preservation
and legacy deep links.
The old suite opts into a development-only `X-Runalytics-Legacy: 1` header;
ordinary requests and all production entry pages load Training Desk.

See [Project Prism](docs/design-system/foundations.md) and its
[migration ledger](docs/design-system/migration-status.md) for the maintained
UI reference and verification evidence. The synthetic preview is available at
`http://localhost:1500/__desk_preview.html` when the **Redesign preview** task runs.
Live COROS, physical iPhone, updater and offline archive operation remain native
acceptance checks, not claims made by the synthetic browser suite.

## Calendar Subscription

A local file import is a snapshot, not a remote subscription. Runalytics can
overwrite one hosted ICS file after activity syncs, plan creation, accepted
adjustments, plan deletion, or a manual calendar refresh. Your calendar app
subscribes to the same HTTPS URL; the last published feed stays available while
Runalytics is closed. New changes are uploaded when the desktop app runs.

In **Settings > Calendar and devices > Calendar subscription**, enter:

1. **Upload URL (PUT)**: the exact HTTPS file destination on your existing host,
  not a folder or CalDAV collection. The parent folder must already exist.
  Use a dedicated file: publishing replaces its entire contents.
2. **Subscription URL (GET)**: a direct HTTPS read URL for that same file,
  accessible without upload credentials, cookies, or an HTML login/share page.
  Use a private, unguessable read URL. Anyone who has it can read your running
  history, schedule, and workout notes. A host providing only authenticated
  WebDAV downloads needs a separate private read link before this works.
3. Optional upload username and password/app password. With no username, the
  secret is sent as a bearer token. Secrets are saved in macOS Keychain, not
  the settings file. A blank password keeps the existing credential; select
  **Remove saved credential** to clear it.
4. Enable **Publish automatically** and save. Runalytics checks that an
  unauthenticated GET returns the uploaded feed. Redirects are not followed;
  enter final URLs. Upload errors appear on the Calendar page and in sync
  results; local data remains intact. Refresh to retry after fixing hosting.
5. After a successful publication, use **Subscribe**, or copy the HTTPS URL
  into your calendar app's subscription / "From URL" option. Remove any
  previously imported calendar copy to avoid duplicates. Calendar clients
  control refresh frequency; updates are not necessarily immediate.

Publishing is off by default. Turning it off stops uploads but does **not**
remove the hosted file or revoke its private URL; do that on your host.

Completion matching is running-only, one-to-one, on the Mac's local date. It
automatically matches only an unambiguous day with one unmatched planned
session and one unmatched run, after plan creation, within 50-150% of planned
distance. For double days, shortened runs, or rescheduled sessions, open the
planned workout and select a recorded activity from the surrounding week.
The planned event's UID is retained and its revision increases, so subscribers
see a replacement rather than a second event. Original notes survive aborting
the plan. Other recorded runs appear as standalone completed events.

Workout details use the same plan-aware breakdown as the generator: warm-up,
repetitions or tempo blocks, jog recoveries between repetitions, and cool-down.
The session pattern depends on the race distance and stage of the plan.
Pace targets include ranges of at least +/-5 seconds/km; easy running uses a
wider range. These are pace ranges, not watch HR zones.

New plans rotate 400/600/800 m repetitions with continuous, cruise, and progressive
tempos. The second half of half-marathon and marathon builds favours tempo work
and introduces 1,000/1,600 m repetitions. Recovery weeks are easy, and quality
sessions are placed at least two days from the long run. The final pre-race week
uses short repetitions; race week has no separate quality session.

Repetition pace scales with length (predicted 3K, 5K, or 10K pace), with shorter
reps substituted when pace or mileage makes longer ones unsuitable. Fast running
is capped at 25 minutes for intervals and 30 minutes for tempos. Structured
sessions have a 1.5 km warm-up and 1 km cool-down, both checked as 500 m multiples.
Recoveries occur only between repetitions. Unused quality mileage is reassigned
to easy runs, so generated workout blocks and weekly totals agree. Sessions under
3.5 km become easy runs.

Session selection is deterministic from the saved race date, creation date, and
race distance, shared by plan generation and calendar export. Regenerate an
existing plan to adopt the new scheduling and mileage allocation; saved schedules
are not rewritten automatically.

The Runna calendar reviewed on 2026-10-19 contained no events before 2025-05-20
(its history starts in July 2025). Before 2026-04-26 it contained 140 events, mostly
completed runs rather than recoverable planned prescriptions. Retained descriptions
included rolling 400s, progressive runs, 1 km repeats, and 1 km over/unders. These
support using varied workout formats, but do not establish either requested build's
complete planned sequence. The rotation and effort caps above are Runalytics
heuristics, not a reconstruction of missing Runna plans.

## Installing on iPhone

Every release carries an iOS build next to the macOS DMGs: the asset named
`Runalytics_<version>.ipa`. It is not on the App Store and it is not
notarised, so it installs through [Sideloadly], which signs it on your
machine with **your own** Apple ID:

1. Download `Runalytics_<version>.ipa` from the [releases] page.
2. Open Sideloadly, drag the IPA onto it, and enter your Apple ID when asked.
   Leave the App ID and signing identity at what Sideloadly suggests.
3. Click **Start**. When it reports the app as installed, open it on the phone.
4. If it will not open: **Settings → General → VPN & Device Management**,
   choose your Apple ID under *App Account*, tap **Trust**, then reopen the
   app. On iOS 16 and later you also need **Settings → Privacy & Security →
   Developer Mode** switched on, because Sideloadly signs with a development
   certificate.

With a free Apple ID the signature lasts **seven days**. Sideloadly can renew it
for you — enable its automatic app refreshing, which re-signs in the background
before the window closes — otherwise the app stops launching until you run
Sideloadly again with the same IPA and Apple ID. Either way nothing is
reinstalled, so the plan and synced history on the phone survive. A paid Apple
Developer account signs for a year.

Free Apple IDs are also rate-limited — Apple caps how many sideloaded apps a
device holds at once and how many App IDs an account can mint per seven days —
so uninstall what you no longer use before adding another.

The IPA is built unsigned on purpose. A development-signed build only launches
on the specific devices listed in its provisioning profile, so it would be
useless to anyone but the machine that produced it; an unsigned bundle is the
raw app, which Sideloadly can sign for whoever installs it. This is the iOS
counterpart of the ad-hoc-signed DMG: both are sealed by the person who
installs them, and both need one manual override on first launch.

To get an existing plan onto the phone rather than recreating it there, see
**Plan sync** in Settings. Both devices must use the same COROS account and
the same GitHub account's Gist channel. The Gist is authoritative: returning
to the app checks it independently of COROS activity downloads, and an open,
online app checks every minute. **Check now** requests a refresh immediately.

Local plan edits remain pending while offline. Before uploading, the app
checks their acknowledged remote base. If the Gist changed meanwhile, its
snapshot replaces the conflicting local plan changes automatically. Unrelated
activities, health records, pain entries and form drafts are retained. Ended
plans publish an explicit empty state so another device cannot resurrect them.

Update both apps before relying on this policy: old versions can still upload
stale snapshots, and cannot read the new plan-ending format. On the first check
after upgrading, an existing Gist wins over an untracked local plan. Missing pinned,
unauthorized, unreadable or ambiguous channels block uploads rather than silently
creating a replacement. GitHub Gist does not document atomic conditional writes;
pre-upload checks and read-back verification cannot eliminate every simultaneous
write race. Offline or suspended apps cannot stay continuously current.

## Development

```sh
npm install
npm run tauri dev     # vite + tauri hot reload
```

Rust tests (plan engine):

```sh
cd src-tauri && cargo test
```

Release bundle (`.app` + `.dmg`):

```sh
npm run build:dmg
```

`tauri build` produces the `.app`; the DMG is built with
`scripts/make_dmg.sh` (`hdiutil makehybrid`) instead of Tauri's DMG step,
because endpoint-security agents (e.g. CrowdStrike Falcon) hold the
interstitial mount busy and make `bundle_dmg.sh` fail with
`hdiutil: create failed - Resource busy`.

### Cutting a release

One command, CI does the build — bumps the version, commits, tags, and
pushes; `.github/workflows/release.yml` then builds both Apple-silicon and
Intel DMGs plus the iOS IPA and creates a **draft** release (publish it from
the Releases page once all three jobs are green):

```sh
./scripts/tag_release.sh 0.2.0
```

Or build and publish from your machine:

```sh
./scripts/bump_version.sh 0.2.0   # update all five version declarations
./scripts/release.sh v0.2.0       # commit + tag + build + gh release
```

`bump_version.sh` rewrites package.json, package-lock.json,
`src-tauri/tauri.conf.json`, `Cargo.toml` and `Cargo.lock`, then verifies
every field. `release.sh` commits the bump, tags and pushes, runs
`npm run tauri build` + `make_dmg.sh` + `build_ios.sh`, and publishes the DMG
and IPA as a GitHub release via `gh` (requires the `gh` CLI). Pass `--dry-run`
to either script to preview the steps without committing or publishing. Set
`SKIP_IOS=1` to skip the IPA on a machine without Xcode — CI publishes it for
that version instead.

The two paths publish the same tag — if CI created the release, don't
also run `release.sh` for that version (or delete the CI draft with
`gh release delete v0.2.0 -y` first). No signing secret is needed:
the app is ad-hoc-signed (`bundle.macOS.signingIdentity: "-"` in
`tauri.conf.json`), which seals the bundle so macOS opens it, but it is
not notarised — on first launch macOS warns the developer is
"unidentified", so users install with right-click → Open (once per
version). `make_dmg.sh` refuses to build a DMG around a bundle whose
signature does not verify, so an unsigned build fails the release
instead of shipping a "damaged" app.

The iOS job is the mirror image of that rule. `scripts/build_ios.sh` runs
`tauri ios build --no-sign`, and refuses any IPA that *is* signed — a
development-signed build would install only on the handful of devices in its
provisioning profile, and would fail silently on every recipient's phone
rather than at build time. Recipients re-sign with their own Apple ID through
Sideloadly, so no certificate or App Store Connect credential is involved on
either side. The build needs no `tauri ios init` and no CocoaPods: the Xcode
scaffold is committed under `src-tauri/gen/apple`, the Podfile declares zero
pods, and the CLI only shells out to `pod install` from `init`.

## Architecture

| Piece | Where | Notes |
| --- | --- | --- |
| MCP client / OAuth | `src-tauri/src/providers/coros.rs` | rmcp streamable-HTTP + keychain `CredentialStore`, loopback OAuth callback |
| Storage | `src-tauri/src/store.rs` | SQLite (WAL, `user_version` migrations) in app data dir |
| FIT decoding | `src-tauri/src/fit.rs` | rustyfit streaming decoder → summary + track points |
| Plan engine | `src-tauri/src/plan/` | deterministic generator + conservative adaptive review |
| Scores | `src-tauri/src/metrics.rs` | consistency + ACWR + recovery heuristics |
| ICS export | `src-tauri/src/calendar.rs` | hand-rolled RFC 5545 emitter |
| Calendar publishing | `src-tauri/src/calendar_publish.rs` | HTTPS PUT, separate read URL, Keychain credentials |
| Plan sync | `src-tauri/src/plan_sync.rs` | AES-GCM blob in a private gist, key derived from the COROS account |
| Frontend | `src/desk/` + `desk.html` | React/TypeScript workspaces; existing IPC and types; lazy MapLibre maps |
| Entry compatibility | `vite.config.ts` + `src/desk/data.tsx` | original HTML URLs build to the React entry; legacy hashes map to the appropriate workspace |

Data lives in `~/Library/Application Support/com.fradone.runalytics/`
(SQLite DB, `settings.json`, `runalytics.ics`).

## Design And Presentation

Training Desk replaces the former page hierarchy instead of reskinning it.
Its source is independent of the old controllers, navigation, overlays and
page styles. Original HTML and controller files remain in the working tree
for regression comparison; Vite substitutes `desk.html` before processing
each production entry, so the old presentation is not bundled into those pages.

- `src/desk/tokens.css` owns semantic typography, materials, spacing and motion.
- `src/desk/desk.css` owns responsive layout; `interaction.css` and
  `interaction.tsx` share control states, selection and surface physics.
- `src/desk/ui.tsx` owns buttons, feedback, native dialogs and inspectors.
  Inspectors remain nonmodal on desktop; on mobile they contain keyboard focus,
  make the background inert, prevent background scrolling and restore focus.
- `src/desk/data.tsx` owns hash routing, refresh state and native progress events.
  Background refresh preserves mounted forms and conversations. Settings patches
  merge into the latest backend value instead of replacing unrelated fields.
- `src/desk/training.tsx`, `journal.tsx`, `health.tsx` and `settings.tsx` own the
  workspaces. Rust and the existing `src/lib/ipc.ts` remain authoritative for
  training decisions, authentication, persistence and external publishing.

The three considered concepts and selected Architectural Spatial Glass direction
are in [Prism foundations](docs/design-system/foundations.md). The
[migration ledger](docs/design-system/migration-status.md) records executed checks
and remaining acceptance limits; historical screenshots are not current evidence.

### Preview without the desktop app

The development-only preview injects synthetic IPC data so all replacement
workspaces can be explored in a browser without an account:

```sh
npx vite --host 127.0.0.1 --port 1501 --strictPort
# then open:
#   http://127.0.0.1:1501/__desk_preview.html
#   http://127.0.0.1:1501/__desk_preview.html?theme=dark#health
```

Use that command rather than `npm run dev`, whose `predev` step kills whatever
holds port 1499.

Normal pages require the native Tauri bridge for real data. The synthetic
preview and legacy test override are not included in production builds.

### Known Release Checks

The production MapLibre map remains a lazy-loaded chunk of about 1.05 MB
minified, and Vite reports its bundle-size warning. `npm audit --omit=dev`
also reports the existing critical MapLibre sanitizer advisory
[GHSA-jrc7-96c5-q579](https://github.com/advisories/GHSA-jrc7-96c5-q579).
The suggested fix upgrades MapLibre from v5 to v6; that dependency migration
was not included in this presentation redesign and remains a release risk.

[COROS MCP server]: https://mcp.coros.com/mcp
[PMTiles]: https://pmtiles.org
[Sideloadly]: https://sideloadly.io
[releases]: https://github.com/francescodone/runalytics/releases
