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
  Desktop uses day lanes with a contextual session inspector; mobile uses a
  date ribbon, daily itinerary and full-screen detail. Connection and initial
  sync actions appear when needed.
- **Journal**: searchable activity history with sport, source and date filters;
  selected-run routes, recorded laps, kilometre splits, prescribed targets and
  stored load/risk impact. A separate training map shows all recorded routes.
- **Health**: readiness, recovery, training load, injury risk, fitness, wearable
  history, pain episodes, reviewed adjustments and consistency milestones.
- **Pace insights**: Health > Pace insights compares the last five eligible
  Speed (interval/tempo) or pace-targeted Long sessions. The chart shows each
  session's fastest and slowest target-relative work reps, with an inspectable
  rep table, outdated-session indicators, activity links and the next workout.
  Pace changes still use the existing review-and-approve workflow.
- **Navigation**: Training, Journal and Health are the three primary workspaces.
  Desktop has a top application bar; mobile has three bottom destinations.
  Settings and Help are header utilities. Settings uses a searchable directory
  and focused task editors, not a second sidebar. The independent React
  presentation pairs locally bundled Source Serif 4 and Source Sans 3 with
  translucent rounded chrome, stronger palette tints and restrained spring
  transitions. Charts retain green evidence colors and fading fills. Reduced
  motion, reduced transparency and increased contrast have explicit fallbacks.
  Both fonts are OFL-licensed; appearance and palette preferences remain available.
- **Plans**: 5K / 10K / half / marathon plans to a target date from your
  threshold pace, weekly frequency, long-run day, and mileage-increase method
  (+5 / +8 / +12 % per week). Geometric ramp with 3-build/1-down cycles,
  taper, and race week.
  Creation uses Direction, Baseline and Review steps. The plan horizon combines
  a week-by-week roadmap with inputs, predictions, publishing and management.
  Creation is unavailable while a plan is active; ending it requires confirmation.
- **Adaptive adjustments**: after every sync, poor recovery (sleep / RHR / HRV
  / ACWR) or a struggled session softens the next hard workout — never the
  other way around.
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
- **Pain journal**: a dedicated Health view supports region selection, intensity,
  onset, notes, follow-up, resolve/reopen and confirmed deletion. Proposed plan
  changes require review. Optional assistant conversations require consent and
  can be cleared separately from the pain record.
- **Watch connection**: Settings shows the next setup step or a direct Sync now
  action for connected accounts. Connection diagnostics remain collapsed.
- **Heatmap**: MapLibre GL JS with a heatmap layer over OSM raster tiles, or
  fully offline from a [PMTiles] archive (path configurable in Settings).
- **Plan sync**: the desktop app publishes the active plan to a private GitHub
  gist that the phone finds and imports, keyed to your COROS account. Enable it
  with the **Plan sync** toggle in Settings. One-way, desktop to phone;
  completed workouts stay local.
- **Garmin**: feature-gated (`--features garmin`); no official public API
  exists, so it ships as a stub.

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
normal-or-better recovery, injury risk below 60%, no unresolved pain and a finite
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

Readiness, recovery, training and injury-risk scores use fixed 0-100% axes, not
the minimum and maximum of the selected period. Readiness history uses the same
45% training + 35% recovery + 20% inverse-risk formula as the current score.
Normalized training load is `100 * raw ratio / configured optimized-load ceiling`,
clamped to 0-100%. Values above the ceiling remain 100%; the unmodified ratio and
short-/long-term load remain visible. Missing values are not rendered as zero.

### Browser Checks

`npm run test:ui` runs the Playwright regression suites using synthetic
harness data and an installed Google Chrome. It starts Vite on port 1500 if no
preview is already running. `npm run build` checks TypeScript and production
page entries, including Support. Screenshots and failure traces go into the
ignored `test-results/` directory.

The original redesign acceptance passed 89 tests. On 31/10/2026, a full 97-test
run passed 87; six mobile visual-matrix tests and four legacy daily-signal tests
failed because they require a workout today while the Saturday fixture is a
rest day. Those existing date-dependent assumptions remain unchanged.

After the adaptation/material changes, production build and 90 selected browser
tests pass out of 100 total, excluding those ten previously diagnosed cases.
Run that verification with `npm run build && npx playwright test --grep-invert
'visual matrix (light|dark) (320|390|768)px|daily signal labels'`.
The updated animated mobile-inspector geometry test also passes independently.
Backend verification passes 19 pacing/adaptation, 15 lap and 13 scoring tests.
Replacement coverage includes ten views at 320/390/768/1440px in light/dark
themes, Axe checks, keyboard focus, map pixels/zoom, failed drafts, Undo,
reviewed mutations, credential preservation and legacy deep links.
The old suite opts into a development-only `X-Runalytics-Legacy: 1` header;
ordinary requests and all production entry pages load Training Desk.

See the [architecture comparison and acceptance record](plan/training-desk-redesign.md).
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
**Plan sync** in Settings: the desktop app publishes the active plan to a
private GitHub gist that the phone finds and imports.

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

- `src/desk/desk.css` owns the new tokens, responsive layouts and motion rules.
- `src/desk/ui.tsx` owns buttons, feedback, native dialogs and inspectors.
  Inspectors remain nonmodal on desktop; on mobile they contain keyboard focus,
  make the background inert, prevent background scrolling and restore focus.
- `src/desk/data.tsx` owns hash routing, refresh state and native progress events.
  Background refresh preserves mounted forms and conversations. Settings patches
  merge into the latest backend value instead of replacing unrelated fields.
- `src/desk/training.tsx`, `journal.tsx`, `health.tsx` and `settings.tsx` own the
  workspaces. Rust and the existing `src/lib/ipc.ts` remain authoritative for
  training decisions, authentication, persistence and external publishing.

The complete before/after screen comparison, three considered concepts and
verification evidence are in [the design record](plan/training-desk-redesign.md).

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
