# Runalytics

Training analytics for runners. A Tauri v2 desktop app (macOS) that pulls your
COROS data, scores your training, builds adaptive race-preparation plans, and
shows where you run.

## Features

- **COROS integration** over the official [COROS MCP server]
  (`https://mcp.coros.com/mcp`, OAuth 2.0 + PKCE). Connect opens your browser;
  tokens live in the macOS keychain and auto-refresh.
- **Sync** activities (with FIT tracks), daily health (sleep, HRV, resting HR,
  stress), fitness assessment (VO₂max, threshold, predicted times) and
  training load.
- **Dashboard**: training score, injury risk, recovery score, and a weighted
  global score, plus fitness cards and recent activities.
- **Plans**: 5K / 10K / half / marathon plans to a target date from your
  threshold pace, weekly frequency, long-run day, and mileage-increase method
  (+5 / +8 / +12 % per week). Geometric ramp with 3-build/1-down cycles,
  taper, and race week.
- **Adaptive adjustments**: after every sync, poor recovery (sleep / RHR / HRV
  / ACWR) or a struggled session softens the next hard workout — never the
  other way around.
- **Calendar**: inspect workouts in Plan or Calendar, including warm-up,
  repetitions, recovery, cool-down, and pace ranges. Completed runs replace
  matched planned events and retain the original plan in their notes.
  An ICS feed can be published automatically to your HTTPS/WebDAV host.
  Push workouts to COROS's training schedule from the Plan page.
- **Heatmap**: MapLibre GL JS with a heatmap layer over OSM raster tiles, or
  fully offline from a [PMTiles] archive (path configurable in Settings).
- **Garmin**: feature-gated (`--features garmin`); no official public API
  exists, so it ships as a stub.

## Calendar Subscription

A local file import is a snapshot, not a remote subscription. Runalytics can
overwrite one hosted ICS file after activity syncs, plan creation, accepted
adjustments, plan deletion, or a manual calendar refresh. Your calendar app
subscribes to the same HTTPS URL; the last published feed stays available while
Runalytics is closed. New changes are uploaded when the desktop app runs.

In Calendar, expand **HTTPS / WebDAV publishing** and enter:

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
| Frontend | `index/plan/calendar/settings.html` + `src/*.ts` | vanilla TS + Vite multi-page, MapLibre heatmap |

Data lives in `~/Library/Application Support/com.fradone.runalytics/`
(SQLite DB, `settings.json`, `runalytics.ics`).

## Design & motion

The UI is vanilla TS on a single stylesheet (`src/styles/app.css`) and follows
an Apple fluid-interface vocabulary: motion is a response to a gesture, not
decoration, and everything honours `prefers-reduced-motion`.

Conventions, and the one file that owns each:

- **`src/lib/motion.ts`** — the only place motion preferences and physics live.
  `prefersReducedMotion()` gates every animation; `SPRING.settle` (critically
  damped) is the default and `SPRING.momentum` (bounce 0.18) is used only after
  a flick. `projectEndpoint()` / `nearestSnap()` carry a drag's velocity into
  the release so a fling continues rather than stopping at the finger.
  `scrollIntoViewMotion()` / `scrollToMotion()` replace raw `scrollIntoView` so
  programmatic scrolls respect reduced motion. `animatePresent` /
  `animateDismiss` / `animateSlideIn` are the shared entrance/exit moves.
- **`src/lib/overlay.ts`** — one modality architecture built on native
  `<dialog>` (`showModal()` supplies the focus trap, Escape, backdrop, and
  focus restore). `createSheet()` is the edge sheet with drag-to-dismiss
  (velocity handoff + rubber-banding); `createModal()` is the centred card with
  a scale/fade present. Activity detail, workout detail, and onboarding all use
  these — there is no hand-rolled focus trap anywhere.
- **`src/lib/feedback.ts`** — `withBusy()` drives a button through its async
  work (disabled, spinner, live stage/percent label) and restores it;
  `confirmSaved()` flashes a "Saved" state. Every long action (sync, push,
  save) uses these instead of ad-hoc `disabled = true`.
- **`src/lib/segmented.ts`** — segmented controls paint a selection pill that
  slides along the track (CSS spring transition, driven by the `.active` class)
  plus a pointer pill that trails the cursor. The theme picker is a
  `role="radiogroup"` on top of this, so arrow keys move the choice itself.

Rules that keep it uniform:

- Press feedback is a `scale(0.96)` transform; entrance motion is a first-paint
  affordance only, so a mid-session refresh shows final values rather than
  replaying the reveal.
- Modals are native `<dialog>`; never build a focus trap or Escape handler by
  hand.
- Reduced motion means snap or cross-fade with no travel — verified at load and
  when toggled mid-motion.
- The emerald chart scale (`--scale-1..5`) is a fixed data ramp and does not
  change with theme or motion work.

### Preview without the desktop app

Harnesses stub `window.__TAURI_INTERNALS__` so the pages render in a plain
browser. Serve the repo and open a harness:

```sh
npx vite --host 127.0.0.1 --port 1501 --strictPort
# then open, e.g.:
#   http://127.0.0.1:1501/__harness.html          (dashboard)
#   http://127.0.0.1:1501/__settings_harness.html (settings)
#   http://127.0.0.1:1501/__injury_test.html      (injuries)
```

Use that command rather than `npm run dev`, whose `predev` step kills whatever
holds port 1499.

[COROS MCP server]: https://mcp.coros.com/mcp
[PMTiles]: https://pmtiles.org
